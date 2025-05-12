use std::{collections::HashMap, env, hash::Hash};

use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::serde_helpers::quantity::vec,
};
use async_trait::async_trait;
use futures::StreamExt;
use tycho_client::feed::component_tracker::ComponentFilter;
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
};

use crate::{
    core::helpers::cpname,
    data::keys,
    types::{
        config::EnvConfig,
        maker::{CompReadjustment, IMarketMaker, Inventory, MarketMaker, TradeDirection},
        tycho::{PsbConfig, SharedTychoStreamState, SrzToken},
    },
    utils::r#static::{ADD_TVL_THRESHOLD, BASIS_POINT_DENO, NULL_ADDRESS},
};

#[async_trait]
impl IMarketMaker for MarketMaker {
    /// Market Maker main functions

    async fn market_price(&self) -> Result<f64, String> {
        self.feed.get(self.config.clone()).await
    }

    /// Get the prices of the components
    fn prices(&self, components: &[ProtocolComponent], pts: &HashMap<String, Box<dyn ProtocolSim>>) -> Vec<f64> {
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
                proto.spot_price(&cp.tokens[1], &cp.tokens[0])
            } else {
                proto.spot_price(&cp.tokens[0], &cp.tokens[1])
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
        if sps.len() == 0 || components.len() != sps.len() {
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
                tracing::debug!(" - Pool {} | Spot: {:>15.5} | Spread: {:>10.2} = {:>10.2} bps", cpname(comp.clone()), spot, spread, spread_bps);
                match spread_bps > 0. {
                    true => {
                        // Pool is above the reference price, sell on pool
                        orders.push(CompReadjustment {
                            component: comp.clone(),
                            direction: TradeDirection::Buy,
                            spot,
                            reference,
                            spread_bps,
                        });
                    }
                    false => {
                        // Pool is below the reference price, buy on pool
                        orders.push(CompReadjustment {
                            component: comp.clone(),
                            direction: TradeDirection::Sell,
                            spot,
                            reference,
                            spread_bps,
                        });
                    }
                };
            }
        }
        // Order by spread
        orders.sort_by(|a, b| {
            if a.spread_bps > b.spread_bps {
                std::cmp::Ordering::Greater
            } else if a.spread_bps < b.spread_bps {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });
        // Compensation evaluation too ?
        orders
    }

    /// Token inventory balances and metadata
    /// Might take some delay to get the balances which is an problem to deal with later
    /// Should be stored in memory and updated after each readjustment only
    async fn inventory(&self, env: EnvConfig) -> Result<Inventory, String> {
        tracing::debug!("Inventory evaluation | MM at {}", env.wallet_public_key);
        let provider = ProviderBuilder::new().on_http(self.config.rpc.clone().parse().expect("Failed to parse RPC_URL"));
        let tokens = vec![self.base.clone(), self.quote.clone()].iter().map(|t| t.address.to_string()).collect::<Vec<String>>();
        match crate::utils::evm::balances(&provider, env.wallet_public_key.clone(), tokens.clone()).await {
            Ok(balances) => {
                tracing::debug!("Balances: {:?}", balances);
                match provider.get_transaction_count(env.wallet_public_key.to_string().parse().unwrap()).await {
                    Ok(nonce) => {
                        tracing::debug!("Nonce of sender {}: {}", env.wallet_public_key.clone(), nonce);
                        Ok(Inventory {
                            base: balances[0].clone(),
                            quote: balances[1].clone(),
                            nonce,
                        })
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get nonce: {:?}", e);
                        Err(e.to_string())
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get inventory: {:?}", e);
                Err(e.to_string())
            }
        }
    }

    /// Find the optimal size for a given readjustment
    // async fn size() {}

    /// Process readjustment orders
    async fn readjust(&self, inventory: Inventory, crs: Vec<CompReadjustment>) {
        // Profitability
        tracing::debug!("Profitability evaluation: {}", self.config.profitability);
        for cr in crs.iter() {
            if !self.config.profitability {
            } else {
                // tracing::warn!("Profitability evaluation not implemented yet");
                // Fees (LP and gas)
                return;
                // buy until the effective marginal price (marginal price + fee) is equal to the market price.
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
            let tokens = vec![self.base.clone(), self.quote.clone()];
            let psbc = PsbConfig {
                filter: ComponentFilter::with_tvl_range(ADD_TVL_THRESHOLD, ADD_TVL_THRESHOLD),
            };
            let mut components: Vec<tycho_simulation::protocol::models::ProtocolComponent> = vec![];
            let mut protosims: HashMap<String, Box<dyn ProtocolSim>> = HashMap::new();
            let psb = crate::core::helpers::psb(self.config.clone(), env.tycho_api_key.to_string(), psbc.clone(), tokens.clone()).await;
            let _stream = match psb.build().await {
                Ok(mut stream) => loop {
                    // Looping
                    match stream.next().await {
                        Some(msg) => match msg {
                            Ok(msg) => {
                                let time = std::time::SystemTime::now();
                                let reference = self.market_price().await.unwrap_or_default();
                                tracing::info!(
                                    "'{}' stream: block # {} with {} states updates | Market price: {}", // , + {} pairs, - {} pairs",
                                    self.config.network.clone(),
                                    msg.block_number,
                                    msg.states.len(),
                                    reference // msg.new_pairs.len(),
                                              // msg.removed_pairs.len()
                                );
                                if self.ready == false {
                                    // --- First stream ---
                                    protosims = msg.states.clone();
                                    let mut keys = vec![];
                                    for (_id, comp) in msg.new_pairs.iter() {
                                        keys.push(comp.id.to_string().to_lowercase());
                                    }
                                    for k in keys.clone() {
                                        if let Some(_proto) = msg.states.get(&k.to_string()) {
                                            // Need to make sure protosim exists
                                            let comp = msg.new_pairs.get(&k.to_string()).expect("New pair not found");
                                            let symbols = comp.tokens.iter().map(|t| t.symbol.clone()).collect::<Vec<String>>();
                                            if !comp.id.to_string().contains(NULL_ADDRESS) {
                                                tracing::debug!(" - Adding component of type {:<20} | Tokens: {:?} | Id: {:<10}", comp.protocol_type_name, symbols, comp.id);
                                                components.push(comp.clone());
                                            }
                                        }
                                    }
                                    self.ready = true;
                                    tracing::info!("✅ ProtocolStreamBuilder initialised successfully");
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

                                    // --- Evaluate ---
                                    let prices = self.prices(&components, &protosims);
                                    let readjusments = self.evaluate(components.clone(), prices.clone(), reference).await;
                                    if readjusments.len() > 0 {
                                        let inventory = self.inventory(env.clone()).await;
                                        let elasped = time.elapsed().unwrap_or_default().as_millis();
                                        tracing::info!(" - Evaluation until readjustments took {} ms", elasped);
                                        self.readjust(inventory.unwrap(), readjusments).await;
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
