use std::collections::HashMap;

use alloy::providers::{Provider, ProviderBuilder};
use async_trait::async_trait;
use futures::StreamExt;
use tycho_client::feed::component_tracker::ComponentFilter;
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
};

use crate::{
    helpers::global::{cpname, get_component_balances},
    types::{
        config::EnvConfig,
        maker::{CompReadjustment, ExecutionOrder, IMarketMaker, Inventory, MarketContext, MarketMaker, TradeDirection},
        tycho::{ProtoSimComp, PsbConfig, SharedTychoStreamState},
    },
    utils::r#static::{ADD_TVL_THRESHOLD, BASIS_POINT_DENO, NULL_ADDRESS},
};

use super::pricefeed::chainlink;

#[async_trait]
impl IMarketMaker for MarketMaker {
    /// Market Maker main functions

    async fn fetch_market_price(&self) -> Result<f64, String> {
        self.feed.get(self.config.clone()).await
    }

    async fn fetch_eth_usd(&self) -> Result<f64, String> {
        chainlink(self.config.rpc.clone(), self.config.gas_token_chainlink.clone()).await
    }

    /// Get the prices of the components
    fn get_prices(&self, components: &[ProtocolComponent], pts: &HashMap<String, Box<dyn ProtocolSim>>) -> Vec<f64> {
        let mut ss = Vec::new();
        for cp in components.iter() {
            let token0 = cp.tokens[0].address.to_string().to_lowercase();
            let is0base = token0 == self.base.address.to_string().to_lowercase();
            let proto = match pts.get(&cp.id.to_string()) {
                Some(p) => p,
                None => {
                    tracing::warn!("Missing protosim for component {}", cp.id);
                    continue;
                }
            };
            let result = if is0base {
                proto.spot_price(&cp.tokens[0], &cp.tokens[1])
            } else {
                proto.spot_price(&cp.tokens[1], &cp.tokens[0])
            };
            match result {
                Ok(price) => {
                    ss.push(price);
                }
                Err(_) => {
                    tracing::warn!("Failed to get spot price on component {}", cp.id);
                }
            }
        }
        ss
    }

    // Evaluate if given pools are out of range (= require intervention)
    async fn evaluate(&self, components: Vec<ProtocolComponent>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment> {
        let mut orders = vec![];
        if sps.is_empty() || components.len() != sps.len() {
            tracing::warn!("Components and spot prices length mismatch ({} != {})", components.len(), sps.len());
            return vec![];
        }
        // tracing::debug!("Evaluating {} pools...", components.len());
        for (i, comp) in components.iter().enumerate() {
            let spot = sps[i];
            let spread = spot - reference;
            let spread_bps = spread / reference * BASIS_POINT_DENO;
            // Check if the spread is above the threshold
            if spread_bps.abs() > self.config.spread as f64 {
                match spread_bps > 0. {
                    true => {
                        // pool's 'quote' token is above the reference price, sell on pool
                        orders.push(CompReadjustment {
                            component: comp.clone(),
                            direction: TradeDirection::Buy,
                            selling: self.base.clone(),
                            buying: self.quote.clone(),
                            spot,
                            reference,
                            spread,
                            spread_bps,
                        });
                    }
                    false => {
                        // pool's 'quote' token is below the reference price, buy on pool
                        orders.push(CompReadjustment {
                            component: comp.clone(),
                            direction: TradeDirection::Sell,
                            selling: self.quote.clone(),
                            buying: self.base.clone(),
                            spot,
                            reference,
                            spread,
                            spread_bps,
                        });
                    }
                };
            }
        }
        // Compensation evaluation too ?
        orders
    }

    /// Token inventory balances and metadata
    /// Might take some delay to get the balances which is an problem to deal with later
    /// Should be stored in memory and updated after each readjustment only
    async fn fetch_inventory(&self, env: EnvConfig) -> Result<Inventory, String> {
        let provider = ProviderBuilder::new().on_http(self.config.rpc.clone().parse().expect("Failed to parse RPC_URL"));
        let tokens = [self.base.clone(), self.quote.clone()].iter().map(|t| t.address.to_string()).collect::<Vec<String>>();
        match crate::utils::evm::balances(&provider, env.wallet_public_key.clone(), tokens.clone()).await {
            Ok(balances) => match provider.get_transaction_count(env.wallet_public_key.to_string().parse().unwrap()).await {
                Ok(nonce) => {
                    tracing::debug!("Inventory evaluation | Balances: {:?} | Nonce {} | Wallet {}", balances, nonce, env.wallet_public_key);
                    Ok(Inventory {
                        base_balance: balances[0],
                        quote_balance: balances[1],
                        nonce,
                    })
                }
                Err(e) => {
                    tracing::warn!("Failed to get nonce: {:?}", e);
                    Err(e.to_string())
                }
            },
            Err(e) => {
                tracing::warn!("Failed to get inventory: {:?}", e);
                Err(e.to_string())
            }
        }
    }

    /// @param components: list of ALL components, used to find the path than can be multi hop
    /// @param tokens: list of ALL tokens
    /// Fetch base/ETH and quote/ETH spot prices
    /// Fetch ETH/USD
    /// Compute base/USD and quote/USD
    async fn fetch_market_context(&self, ethpts: Vec<ProtoSimComp>, components: Vec<ProtocolComponent>, tokens: Vec<Token>) -> Option<MarketContext> {
        let eth_to_usd = self.fetch_eth_usd().await;
        let base_to_eth_vp = super::routing::find_path(components.clone(), self.base.address.to_string().to_lowercase(), self.config.gas_token.to_lowercase());
        let quote_to_eth_vp = super::routing::find_path(components.clone(), self.quote.address.to_string().to_lowercase(), self.config.gas_token.to_lowercase());
        match (base_to_eth_vp, quote_to_eth_vp, eth_to_usd) {
            (Ok(base_to_eth_vp), Ok(quote_to_eth_vp), Ok(eth_to_usd)) => {
                let base_to_eth = super::routing::quote(ethpts.clone(), tokens.clone(), base_to_eth_vp.token_path.clone());
                let quote_to_eth = super::routing::quote(ethpts.clone(), tokens.clone(), quote_to_eth_vp.token_path.clone());
                match (base_to_eth, quote_to_eth) {
                    (Some(base_to_eth), Some(quote_to_eth)) => Some(MarketContext {
                        base_to_eth,
                        quote_to_eth,
                        eth_to_usd,
                    }),
                    _ => {
                        tracing::warn!("Failed to get base/ETH quote");
                        None
                    }
                }
            }
            _ => {
                tracing::warn!("Failed to find path for base|quote to ETH.");
                None
            }
        }
    }

    /// Find the optimal size for a given readjustment
    // async fn size() {}

    /// Process readjustment orders
    /// Questions, given that there might be multiple readjustments to do:
    /// - How to allocate the size of each readjustment, they are dependent on each other
    /// "Optimal swap is to swap until marginal price + fee = market price"
    async fn readjust(&self, inventory: Inventory, mut crs: Vec<CompReadjustment>, env: EnvConfig) {
        // --- Ordering ---
        // Order by spread
        crs.sort_by(|a, b| {
            if a.spread_bps > b.spread_bps {
                std::cmp::Ordering::Greater
            } else if a.spread_bps < b.spread_bps {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });

        tracing::debug!("Profitability evaluation: {}", self.config.profitability);
        for cr in crs.iter() {
            match get_component_balances(self.config.clone(), cr.component.clone(), env.tycho_api_key.clone()).await {
                Some(balances) => {
                    // for b in balances.iter() {
                    //     tracing::debug!(" - Attribute: {}", b.0);
                    // }

                    let buying = cr.buying.clone();
                    let buying_pow = 10f64.powi(buying.decimals as i32);
                    let pool_buying_balance = balances.get(&buying.address.to_string().to_lowercase()).unwrap_or_else(|| panic!("Failed to get buying balance"));
                    let pool_buying_balance_divided = (*pool_buying_balance as f64) / buying_pow;

                    let selling = cr.selling.clone();
                    let selling_pow = 10f64.powi(selling.decimals as i32);
                    let pool_selling_balance = balances.get(&selling.address.to_string().to_lowercase()).unwrap_or_else(|| panic!("Failed to get selling balance"));
                    let pool_selling_balance_divided = (*pool_selling_balance as f64) / selling_pow;

                    // tracing::debug!(
                    //     " Component {} has {:.2} {} and {:.2} {}",
                    //     cpname(cr.component.clone()),
                    //     pool_buying_balance_divided,
                    //     buying.symbol,
                    //     pool_selling_balance_divided,
                    //     selling.symbol
                    // );

                    // --- Size & Allocation ---
                    let base_to_quote = selling == self.base;
                    let inventory_balance = if selling == self.base { inventory.base_balance } else { inventory.quote_balance };
                    let inventory_balance_divided = (inventory_balance as f64) / selling_pow;
                    let optimal = pool_selling_balance_divided * 1. / BASIS_POINT_DENO;
                    let max_alloc = inventory_balance_divided * self.config.max_trade_allocation;
                    let amount_selling = inventory_balance_divided * self.config.max_trade_allocation;
                    let amount_buying = if base_to_quote { amount_selling * cr.spot } else { amount_selling / cr.spot };

                    let pool_msg = format!(
                        " - Pool {} | Tycho Spot: {:>12.5} vs ref {:>12.5} | Spread: {:>7.2} {} = {:>5.0} bps)",
                        cpname(cr.component.clone()),
                        cr.spot,
                        cr.reference,
                        cr.spread,
                        selling.symbol,
                        cr.spread_bps,
                    );

                    let inventory_msg = format!(
                        " Inventory: {:.2} {} | Optimal: {:.5} | Max: {:.5} | Selling {:.5} {} for {:.5} {}",
                        inventory_balance_divided, selling.symbol, optimal, max_alloc, amount_selling, selling.symbol, amount_buying, buying.symbol
                    );

                    tracing::debug!("{} | {}", pool_msg, inventory_msg);
                    // --- Prepa Exec ---
                    let powered_selling_amount = amount_selling * selling_pow;
                    let powered_buying_amount = amount_buying * buying_pow;
                    let buying_amount_min_recv = amount_buying * (BASIS_POINT_DENO - self.config.slippage as f64) / BASIS_POINT_DENO;
                    let powered_buying_amount_min_recv = buying_amount_min_recv * buying_pow;

                    let exo = ExecutionOrder {
                        cr: cr.clone(),
                        base_to_quote,
                        powered_selling_amount,
                        powered_buying_amount,
                        powered_buying_amount_min_recv,
                    };
                    // --- Gas Fees ---
                    // --- Swap fees ---
                    // --- Profitability ---
                    // --- Prepa execution ---
                }
                None => {
                    tracing::warn!("Failed to get component balances");
                }
            }
        }

        // --- Profitability ---
        // --- Prepa execution ---
        for _cr in crs.iter() {
            if !self.config.profitability {
            } else {
                // tracing::warn!("Profitability evaluation not implemented yet");
                // Fees (LP and gas)
                // Buy until the effective marginal price (marginal price + fee) is equal to the market price.
                return;
            }
        }

        // Max Inventory
        // Allocation
        // Size optimization
        // Execution prepa
    }

    async fn monitor(&mut self, mtx: SharedTychoStreamState, env: EnvConfig) {
        loop {
            tracing::debug!("Connecting ProtocolStreamBuilder for {}", self.config.network);
            let psbc = PsbConfig {
                filter: ComponentFilter::with_tvl_range(ADD_TVL_THRESHOLD, ADD_TVL_THRESHOLD),
            };
            let state = mtx.read().await;
            let atks = state.atks.clone();
            drop(state);
            let mut components = vec![];
            let mut protosims: HashMap<String, Box<dyn ProtocolSim>> = HashMap::new();
            let psb = crate::helpers::global::psb(self.config.clone(), env.tycho_api_key.to_string(), psbc.clone(), atks.clone()).await;
            let _stream = match psb.build().await {
                Ok(mut stream) => loop {
                    // Looping
                    match stream.next().await {
                        Some(msg) => match msg {
                            Ok(msg) => {
                                let time = std::time::SystemTime::now();
                                let reference = self.fetch_market_price().await.unwrap_or_default();
                                tracing::info!(
                                    "'{}' stream: block # {} with {} states updates | Market price: {}", // , + {} pairs, - {} pairs",
                                    self.config.network.clone(),
                                    msg.block_number,
                                    msg.states.len(),
                                    reference // msg.new_pairs.len(),
                                              // msg.removed_pairs.len()
                                );
                                if !self.ready {
                                    // --- First stream ---
                                    protosims = msg.states.clone();
                                    let mut keys = vec![];
                                    for (_id, comp) in msg.new_pairs.iter() {
                                        keys.push(comp.id.to_string().to_lowercase());
                                    }
                                    let mut targets = 0;
                                    for k in keys.clone() {
                                        if let Some(_proto) = msg.states.get(&k.to_string()) {
                                            // Need to make sure protosim exists
                                            let comp = msg.new_pairs.get(&k.to_string()).expect("New pair not found");
                                            let symbols = comp.tokens.iter().map(|t| t.symbol.clone()).collect::<Vec<String>>();
                                            if !comp.id.to_string().contains(NULL_ADDRESS) {
                                                components.push(comp.clone());
                                                // If the component contains both config tokens, add it to the monitored list
                                                let tks = comp.tokens.iter().map(|t| t.address.to_string().to_lowercase()).collect::<Vec<String>>();
                                                if tks.contains(&self.base.address.to_string().to_lowercase()) && tks.contains(&self.quote.address.to_string().to_lowercase()) {
                                                    targets += 1;
                                                    tracing::debug!(" - Adding target component: {} | Tokens: {:?} ", cpname(comp.clone()), symbols);
                                                }
                                            }
                                        }
                                    }
                                    self.ready = true;
                                    tracing::info!("✅ ProtocolStreamBuilder initialised successfully. Monitoring {} on {} components", targets, components.len());
                                } else {
                                    // --- Update protosims ---
                                    if !msg.states.is_empty() {
                                        for x in msg.states.iter() {
                                            protosims.insert(x.0.clone().to_lowercase(), x.1.clone());
                                        }
                                    }
                                    // --- Update new pairs (add or overwrite) ---
                                    for x in msg.new_pairs.iter() {
                                        if let Some(pos) = components.iter().position(|current| current.id.to_string().to_lowercase() == x.0.to_string().to_lowercase()) {
                                            components[pos] = x.1.clone();
                                        } else {
                                            components.push(x.1.clone());
                                        }
                                    }
                                    // --- Remove old pairs ---
                                    for x in msg.removed_pairs.iter() {
                                        if let Some(pos) = components.iter().position(|current| current.id.to_string().to_lowercase() == x.0.to_string().to_lowercase()) {
                                            components.swap_remove(pos);
                                        }
                                    }

                                    // Targets = components with both tokens, to monitor
                                    // Components = all components, used to find route, pricing, etc.
                                    let mut targets = vec![];
                                    for cp in components.iter() {
                                        let tks = cp.tokens.iter().map(|t| t.address.to_string().to_lowercase()).collect::<Vec<String>>();
                                        if tks.contains(&self.base.address.to_string().to_lowercase()) && tks.contains(&self.quote.address.to_string().to_lowercase()) {
                                            targets.push(cp.clone());
                                        }
                                    }

                                    // --- Evaluate ---
                                    let prices = self.get_prices(&targets, &protosims);
                                    let readjusments = self.evaluate(targets.clone(), prices.clone(), reference).await;
                                    if !readjusments.is_empty() {
                                        // This async block should be optimised as much as possible
                                        let inventory = self.fetch_inventory(env.clone()).await;
                                        // let context = self.market_context().await;

                                        let elasped = time.elapsed().unwrap_or_default().as_millis();
                                        tracing::info!(" - Evaluation until readjustments took {} ms", elasped);
                                        self.readjust(inventory.unwrap(), readjusments, env.clone()).await;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Stream error: {:?}", e);
                                break;
                            }
                        },
                        None => {
                            tracing::warn!("Stream closed. Retrying...");
                            break;
                        }
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to build stream on {}: {:?}. Exiting.", self.config.network, e.to_string());
                    return;
                }
            };
            // End
        }
    }
}
