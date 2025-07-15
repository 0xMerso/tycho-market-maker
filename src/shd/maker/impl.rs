use std::{collections::HashMap, str::FromStr};

use crate::{
    maker::tycho::{cpname, get_component_balances},
    opti::routing,
    types::{
        config::EnvConfig,
        maker::{CompReadjustment, ComponentPriceData, ExecutionOrder, IMarketMaker, Inventory, MarketContext, MarketMaker, PreparedTransaction, SwapCalculation, TradeDirection},
        moni::NewPricesMessage,
        tycho::{ProtoSimComp, PsbConfig, SharedTychoStreamState},
    },
    utils::constants::{ADD_TVL_THRESHOLD, APPROVE_FN_SIGNATURE, BASIS_POINT_DENO, DEFAULT_APPROVE_GAS, DEFAULT_SWAP_GAS, NULL_ADDRESS, PRICE_MOVE_THRESHOLD, SHARE_POOL_BAL_SWAP_BPS},
};
use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::{TransactionInput, TransactionRequest},
    sol_types::SolValue,
};

use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use futures::StreamExt;
use num_bigint::BigUint;
use num_traits::cast::ToPrimitive;
use tycho_client::feed::component_tracker::ComponentFilter;
use tycho_execution::encoding::{
    evm::encoder_builder::EVMEncoderBuilder,
    models::{Solution, Transaction},
    tycho_encoder::TychoEncoder,
};
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
};

use alloy_primitives::Bytes as AlloyBytes;

use crate::maker::exec::ExecStrategy;
use crate::maker::feed::PriceFeed;

// Impl print for MarketContext
// Implemented here for tracing/log purposes, prefix path has to be maker.rs file
impl MarketContext {
    pub fn print(&self) {
        tracing::info!(
            "Market Context: Base to ETH: {:.6} | Quote to ETH: {:.6} | ETH to USD: {:.2} | Max Fee per Gas: {} | Max Priority Fee per Gas: {} | Native Gas Price: {} | Block: {:?}",
            self.base_to_eth,
            self.quote_to_eth,
            self.eth_to_usd,
            self.max_fee_per_gas,
            self.max_priority_fee_per_gas,
            self.native_gas_price,
            self.block
        );
    }
}

#[async_trait]
impl<E: ExecStrategy, F: PriceFeed> IMarketMaker for MarketMaker<E, F> {
    /// Market Maker main functions

    async fn fetch_market_price(&self) -> Result<f64, String> {
        self.feed.get(self.config.clone()).await
    }

    async fn fetch_eth_usd(&self) -> Result<f64, String> {
        if self.config.gas_token_chainlink_price_feed.is_empty() {
            tracing::warn!("No gas oracle feed found, using Coingecko");
            return Ok(super::feed::coingecko().await.unwrap_or(0.));
        }
        super::feed::chainlink(self.config.rpc_url.clone(), self.config.gas_token_chainlink_price_feed.clone()).await
    }

    /// Get the prices of the components
    fn prices(&self, psc: &Vec<ProtoSimComp>) -> Vec<ComponentPriceData> {
        let mut ss = Vec::new();
        for proto in psc.iter() {
            let token0 = proto.component.tokens[0].address.to_string().to_lowercase();
            let is0base = token0 == self.base.address.to_string().to_lowercase();
            let protosim = proto.protosim.clone();
            let result = if is0base {
                protosim.spot_price(&proto.component.tokens[0], &proto.component.tokens[1])
            } else {
                protosim.spot_price(&proto.component.tokens[1], &proto.component.tokens[0])
            };
            match result {
                Ok(price) => {
                    ss.push(ComponentPriceData {
                        address: proto.component.id.to_string().to_lowercase(),
                        r#type: proto.component.protocol_system.to_string(),
                        price,
                    });
                }
                Err(_) => {
                    tracing::warn!("Failed to get spot price on component {}", proto.component.id);
                }
            }
        }
        ss
    }

    /// Token inventory balances and metadata
    /// Might take some delay to get the balances which is an problem to deal with later
    /// Should be stored in memory and updated after each readjustment only
    async fn fetch_inventory(&self, env: EnvConfig) -> Result<Inventory, String> {
        let provider = ProviderBuilder::new().on_http(self.config.rpc_url.clone().parse().expect("Failed to parse RPC_URL"));
        let tokens = [self.base.clone(), self.quote.clone()];
        let addresses = tokens.iter().map(|t| t.address.to_string()).collect::<Vec<String>>();
        match crate::utils::evm::balances(&provider, self.config.wallet_public_key.clone(), addresses.clone()).await {
            Ok(balances) => match provider.get_transaction_count(self.config.wallet_public_key.to_string().parse().unwrap()).await {
                Ok(nonce) => {
                    let mut msgs = vec![];
                    for (x, tk) in tokens.iter().enumerate() {
                        let balance = balances.get(x).cloned().unwrap_or_default();
                        let divided = balance as f64 / 10f64.powi(tk.decimals as i32);
                        // tracing::debug!(" - Inventory: Got {} of {}", divided, tk.symbol);
                        msgs.push(format!("{:.3} of {}", divided, tk.symbol));
                    }
                    tracing::debug!("Inventory evaluation: Nonce {} | Wallet {} | 💵 Holding {}", nonce, self.config.wallet_public_key, msgs.join(" and "));
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
    /// ! Compute base/USD and quote/USD, based on a arbitrary path ! Just a valid path !
    async fn fetch_market_context(&self, components: Vec<ProtocolComponent>, protosims: &HashMap<std::string::String, Box<dyn ProtocolSim>>, tokens: Vec<Token>) -> Option<MarketContext> {
        let time = std::time::SystemTime::now();
        match crate::utils::evm::eip1559_fees(self.config.rpc_url.clone()).await {
            Ok(eip1559_fees) => {
                let native_gas_price = crate::utils::evm::gas_price(self.config.rpc_url.clone()).await;
                let eth_to_usd = self.fetch_eth_usd().await;
                let provider = ProviderBuilder::new().on_http(self.config.rpc_url.clone().parse().unwrap());
                let block: alloy::rpc::types::Block = provider.get_block_by_number(alloy::eips::BlockNumberOrTag::Latest, false).await.unwrap().unwrap();
                let base_to_eth_vp = routing::find_path(components.clone(), self.base.address.to_string().to_lowercase(), self.config.gas_token_symbol.to_lowercase());
                let quote_to_eth_vp = routing::find_path(components.clone(), self.quote.address.to_string().to_lowercase(), self.config.gas_token_symbol.to_lowercase());
                match (base_to_eth_vp, quote_to_eth_vp, eth_to_usd) {
                    (Ok(base_to_eth_vp), Ok(quote_to_eth_vp), Ok(eth_to_usd)) => {
                        let mut to_eth_ptss = vec![];
                        for cp in components.iter() {
                            let id = cp.id.to_string().to_lowercase();
                            if base_to_eth_vp.comp_path.contains(&id) || quote_to_eth_vp.comp_path.contains(&id) {
                                match protosims.get(&id) {
                                    Some(protosim) => {
                                        // tracing::debug!("Found paths of size {} | {}", base_to_eth_vp.comp_path.len(), quote_to_eth_vp.comp_path.len());
                                        // tracing::debug!("Found paths : {} | {}", base_to_eth_vp.comp_path.join(","), quote_to_eth_vp.comp_path.join(","));
                                        to_eth_ptss.push(ProtoSimComp {
                                            component: cp.clone(),
                                            protosim: protosim.clone(),
                                        });
                                    }
                                    None => {
                                        tracing::error!("contains: couldn't find protosim for component {}", cp.id);
                                    }
                                }
                            }
                        }
                        let base_to_eth = routing::quote(to_eth_ptss.clone(), tokens.clone(), base_to_eth_vp.token_path.clone());
                        let quote_to_eth = routing::quote(to_eth_ptss.clone(), tokens.clone(), quote_to_eth_vp.token_path.clone());
                        // tracing::debug!("Gas: {:?} | Native: {}", eip1559_fees, native_gas_price);
                        let elasped = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Market context fetched in {} ms", elasped);
                        match (base_to_eth, quote_to_eth) {
                            (Some(base_to_eth), Some(quote_to_eth)) => Some(MarketContext {
                                base_to_eth,
                                quote_to_eth,
                                eth_to_usd,
                                max_fee_per_gas: eip1559_fees.max_fee_per_gas,
                                max_priority_fee_per_gas: eip1559_fees.max_priority_fee_per_gas,
                                native_gas_price,
                                block: block.header.number,
                            }),
                            _ => {
                                tracing::warn!("Failed to get base/ETH quote");
                                None
                            }
                        }
                    }
                    _ => {
                        tracing::error!("Failed to find path for base|quote to ETH.");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch EIP-1559 fees: {:?}", e);
                return None;
            }
        }
    }

    // Evaluate if given pools are out of range (= require intervention)
    // Targets are the pools to monitor, nothing more
    fn evaluate(&self, targets: &Vec<ProtoSimComp>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment> {
        let mut orders = vec![];
        // let mut snapshots = vec![];
        if sps.is_empty() || (targets.len() != sps.len()) {
            tracing::warn!("Components targets and spot prices length mismatch ({} != {})", targets.len(), sps.len());
            return vec![];
        }
        // tracing::debug!("Evaluating {} pools...", targets.len());
        for (i, psc) in targets.iter().enumerate() {
            let spot = sps[i];
            let spread = spot - reference;
            let spread_bps = spread / reference * BASIS_POINT_DENO;
            // Check if the spread is above the threshold
            let symbol = if spread_bps < 0_f64 { "buy 📈" } else { "sell 📉" };
            tracing::debug!(
                "   - Evaluating pool {}: Spot: {:.5} | Reference: {:.5} | Spread: {:.5} | Spread BPS: {:<3.2} | Should {}",
                cpname(psc.component.clone()),
                spot,
                reference,
                spread,
                spread_bps,
                symbol
            );
            // let snapshot = MarketSnapshot {};
            if spread_bps.abs() > self.config.target_spread_bps as f64 {
                match spread_bps > 0. {
                    true => {
                        // pool's 'quote' token is above the reference price, sell on pool
                        orders.push(CompReadjustment {
                            psc: psc.clone(),
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
                            psc: psc.clone(),
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

    /// Process readjustment orders
    /// Questions, given that there might be multiple readjustments to do:
    /// - How to allocate the size of each readjustment, they are dependent on ea
    /// ch other
    /// "Optimal swap is to swap until marginal price + fee = market price"
    async fn readjust(&self, context: MarketContext, inventory: Inventory, mut adjustments: Vec<CompReadjustment>, env: EnvConfig) -> Vec<ExecutionOrder> {
        // --- Ordering ---
        adjustments.sort_by(|a, b| a.spread_bps.partial_cmp(&b.spread_bps).unwrap_or(std::cmp::Ordering::Equal));
        let mut orders = vec![];
        for adjustment in &adjustments {
            let balances_opt = get_component_balances(self.config.clone(), adjustment.psc.component.clone(), env.tycho_api_key.clone()).await;
            let balances = match balances_opt {
                Some(b) => b,
                None => {
                    tracing::warn!("Failed to get component balances");
                    continue;
                }
            };
            // --- Token & Amounts ---
            let buying = &adjustment.buying;
            let buying_pow = 10f64.powi(buying.decimals as i32);
            let buying_addr = buying.address.to_string().to_lowercase();
            let pool_buying_balance = match balances.get(&buying_addr) {
                Some(bal) => bal,
                None => {
                    tracing::warn!("Failed to get buying balance for {}", buying_addr);
                    continue;
                }
            };
            let pool_buying_balance_normalized = (*pool_buying_balance as f64) / buying_pow;
            if pool_buying_balance_normalized < f64::EPSILON {
                tracing::info!("pool_buying_balance_normalized < 0 !");
            }
            let selling = &adjustment.selling;
            let selling_pow = 10f64.powi(selling.decimals as i32);
            let selling_addr = selling.address.to_string().to_lowercase();
            let pool_selling_balance = match balances.get(&selling_addr) {
                Some(bal) => bal,
                None => {
                    tracing::warn!("Failed to get selling balance for {}", selling_addr);
                    continue;
                }
            };
            let pool_selling_balance_normalized = (*pool_selling_balance as f64) / selling_pow;
            if pool_selling_balance_normalized < f64::EPSILON {
                tracing::warn!("Cannot readjust, skipping due to pool_selling_balance_normalized < 0 !");
                continue;
            }

            // Optimum:

            let base_to_quote = *selling == self.base;
            let inventory_balance = if base_to_quote { inventory.base_balance } else { inventory.quote_balance };
            let inventory_balance_normalized = (inventory_balance as f64) / selling_pow;
            let optimal = pool_selling_balance_normalized * SHARE_POOL_BAL_SWAP_BPS / BASIS_POINT_DENO;
            let max_alloc = inventory_balance_normalized * self.config.max_inventory_ratio;
            let selling_amount = max_alloc; // For testing
            let buying_amount = if base_to_quote { selling_amount * adjustment.spot } else { selling_amount / adjustment.spot };
            let pool_msg = format!(
                " - Pool {} | Tycho Spot: {:>12.5} vs ref {:>12.5} | Spread: {:>7.2} {} = {:>5.0} bps",
                cpname(adjustment.psc.component.clone()),
                adjustment.spot,
                adjustment.reference,
                adjustment.spread,
                self.quote.symbol,
                adjustment.spread_bps,
            );
            let inventory_msg = format!(
                " - Inventory: {:.2} {} | Optimal: {:.} | Max: {:.5} | Selling {:.5} {} for {:.5} {}",
                inventory_balance_normalized, selling.symbol, optimal, max_alloc, selling_amount, selling.symbol, buying_amount, buying.symbol
            );
            tracing::debug!("{} | {}", pool_msg, inventory_msg);
            let powered_selling_amount = selling_amount * selling_pow;
            let powered_selling_amount_bg = BigUint::from(powered_selling_amount.floor() as u128);
            let powered_buying_amount = buying_amount * buying_pow;
            let (selling_amount_worth_eth, buying_amount_worth_eth) = if base_to_quote {
                (selling_amount * context.base_to_eth, buying_amount * context.quote_to_eth)
            } else {
                (selling_amount * context.quote_to_eth, buying_amount * context.base_to_eth)
            };
            let (selling_amount_worth_usd, buying_amount_worth_usd) = (selling_amount_worth_eth * context.eth_to_usd, buying_amount_worth_eth * context.eth_to_usd);
            match adjustment.psc.protosim.get_amount_out(powered_selling_amount_bg.clone(), selling, buying) {
                Ok(result) => {
                    let amount_out_powered = result.amount.to_f64().unwrap_or(0.0);
                    let amount_out_normalized = amount_out_powered / 10f64.powi(buying.decimals as i32);
                    let slippage_bps = self.config.max_slippage_pct * BASIS_POINT_DENO;
                    let amount_out_min_normalized = amount_out_normalized * (BASIS_POINT_DENO - slippage_bps) / BASIS_POINT_DENO;
                    let amount_out_min_powered = amount_out_min_normalized * buying_pow;
                    let gas_units = result.gas.to_string().parse::<u128>().unwrap_or_default();
                    let gas_cost_eth = (gas_units.saturating_mul(context.native_gas_price)) as f64 / 1e18;
                    let gas_cost_usd = gas_cost_eth * context.eth_to_usd;
                    let gas_cost_in_output = if base_to_quote { gas_cost_eth / context.quote_to_eth } else { gas_cost_eth / context.base_to_eth };
                    tracing::debug!(
                        " - Swap: {:.5} {} for {:.5} {} | Gas cost : {:.5} $ | Gas cost in output: {:.2} %",
                        selling_amount,
                        selling.symbol,
                        amount_out_normalized,
                        buying.symbol,
                        gas_cost_usd,
                        gas_cost_in_output * 100.0
                    );
                    let average_sell_price = if base_to_quote {
                        amount_out_normalized / selling_amount
                    } else {
                        1. / (amount_out_normalized / selling_amount)
                    };
                    let delta = average_sell_price - adjustment.spot;
                    let price_impact_bps = ((delta / adjustment.spot) * BASIS_POINT_DENO).round();
                    let average_sell_price_net_gas = if base_to_quote {
                        (amount_out_normalized - gas_cost_in_output) / selling_amount
                    } else {
                        1. / ((amount_out_normalized - gas_cost_in_output) / selling_amount)
                    };
                    let delta_net_of_gas = average_sell_price_net_gas - adjustment.spot;
                    let price_impact_net_of_gas_bps = ((delta_net_of_gas / adjustment.spot) * BASIS_POINT_DENO).round();
                    let potential_profit_delta = if base_to_quote {
                        average_sell_price_net_gas - adjustment.reference
                    } else {
                        adjustment.reference - average_sell_price_net_gas
                    };
                    let potential_profit_delta_spread_bps = potential_profit_delta / adjustment.reference * BASIS_POINT_DENO;
                    let profitable = potential_profit_delta_spread_bps > self.config.min_exec_spread_bps;
                    tracing::debug!(
                        " - Profit: {}  with average_sell_price_net_gas: {:.4} vs reference_price: {:.4} | potential_profit_delta: {:.5} | 👀  potential_profit_delta_spread_bps: {:.2}",
                        if potential_profit_delta > 0. { "🟩" } else { "🟧" },
                        average_sell_price_net_gas,
                        adjustment.reference,
                        potential_profit_delta,
                        potential_profit_delta_spread_bps
                    );
                    if profitable {
                        let calculation = SwapCalculation {
                            base_to_quote,
                            selling_amount,
                            buying_amount,
                            powered_selling_amount,
                            powered_buying_amount,
                            amount_out_normalized,
                            amount_out_powered,
                            amount_out_min_normalized,
                            amount_out_min_powered,
                            gas_units,
                            average_sell_price,
                            average_sell_price_net_gas,
                            gas_cost_eth,
                            gas_cost_usd,
                            gas_cost_in_output_token: gas_cost_in_output,
                            selling_worth_usd: selling_amount_worth_usd,
                            buying_worth_usd: buying_amount_worth_usd,
                            profit_delta_bps: potential_profit_delta_spread_bps,
                            profitable,
                        };
                        let order = ExecutionOrder {
                            adjustment: adjustment.clone(),
                            calculation,
                        };
                        orders.push(order);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to simulate get amount out: {:?}", e);
                    continue;
                }
            }
        }
        orders
    }

    /// Build a Tycho Solution struct, for the given order
    async fn solution(&self, order: ExecutionOrder, env: EnvConfig) -> Solution {
        let split = 0.;
        let input = order.adjustment.selling.address;
        let output = order.adjustment.buying.address;

        let amount_in = BigUint::from((order.calculation.powered_selling_amount).floor() as u128);
        let amount_out = BigUint::from((order.calculation.amount_out_powered).floor() as u128);
        let amount_out_min = BigUint::from((order.calculation.amount_out_min_powered).floor() as u128);

        tracing::debug!(
            " - {} : Building Tycho solution: Buying {} with {} | Amount in: {} | Amount out: {} | Amount out min: {} => {} of {}",
            cpname(order.adjustment.psc.component.clone()),
            order.adjustment.buying.symbol,
            order.adjustment.selling.symbol,
            amount_in,
            amount_out,
            amount_out_min,
            order.calculation.amount_out_min_normalized,
            order.adjustment.buying.symbol
        );
        let swap = tycho_execution::encoding::models::Swap::new(order.adjustment.psc.component.clone(), input.clone(), output.clone(), split);
        // tracing::debug!(" - Swap: {:?}", swap);
        // Swap { component: ProtocolComponent { id: "88e6a0c2ddd26feeb64f039a2c41296fcb3f5640", protocol_system: "uniswap_v3", protocol_type_name: "uniswap_v3_pool", chain: Ethereum, tokens: [Bytes(0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48), Byte (0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2)], contract_addresses: [], static_attributes: {"tick_spacing": Bytes(0x0a), "fee": Bytes(0x01f4), "pool_address": Bytes(0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640)}, change: Update, creation_tx: Bytes(0x125e0b641d4a4b08806bf52c0c6757648c9963bcda8681e4f996f09e00d4c2cc), created_at: 2021-05-05T21:42:11 }, token_in: Bytes(0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2), token_out: Bytes(0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48), split: 0.0
        Solution {
            // Addresses
            sender: tycho_simulation::tycho_core::Bytes::from_str(self.config.wallet_public_key.to_lowercase().as_str()).unwrap(),
            receiver: tycho_simulation::tycho_core::Bytes::from_str(self.config.wallet_public_key.to_lowercase().as_str()).unwrap(),
            given_token: input.clone(),
            checked_token: output.clone(),
            // Others fields
            given_amount: amount_in.clone(),
            slippage: Some(self.config.max_slippage_pct), // Slippage in decimal < 1, because 1.0 = 100%
            exact_out: false,                             // It's an exact in solution
            expected_amount: Some(amount_out),
            checked_amount: Some(amount_out_min), // The amount out will not be checked in execution
            swaps: vec![swap.clone()],
            ..Default::default()
        }
    }

    /// Convert a solution to a transaction payload
    /// Also build the approval transaction, presumed needed (never infinite approval)
    /// We assume the bot always need to approve the router, so we don't need to check if it's already approved. Execution might be done in bundle
    fn encode(&self, solution: Solution, tx: Transaction, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Result<PreparedTransaction, String> {
        let max_priority_fee_per_gas = context.max_priority_fee_per_gas; // 1 Gwei, not suited for L2s.
        let max_fee_per_gas = context.max_fee_per_gas;

        // 1. Approvals (Tycho router) with Permit2
        let amount: u128 = solution.given_amount.clone().to_string().parse().expect("Couldn't convert given_amount to u128"); // ?
        let args = (Address::from_str(&self.config.permit2_address).expect("Couldn't convert permit2 to address"), amount);
        let data = tycho_execution::encoding::evm::utils::encode_input(APPROVE_FN_SIGNATURE, args.abi_encode());
        let sender = solution.sender.clone().to_string().parse().expect("Failed to parse sender");
        let approval = TransactionRequest {
            to: Some(alloy::primitives::TxKind::Call(solution.given_token.clone().to_string().parse().expect("Failed to parse given_token"))),
            from: Some(sender),
            value: None,
            input: TransactionInput {
                input: Some(AlloyBytes::from(data)),
                data: None,
            },
            gas: Some(DEFAULT_APPROVE_GAS),
            chain_id: Some(self.config.chain_id),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            nonce: Some(inventory.nonce),
            ..Default::default()
        };

        // 2. Swap --- No bribe for now ---
        let swap = TransactionRequest {
            to: Some(alloy_primitives::TxKind::Call(Address::from_slice(&tx.to))),
            from: Some(self.config.wallet_public_key.parse().expect("Failed to parse wallet public key")),
            value: Some(U256::from(0)),
            input: TransactionInput {
                input: Some(AlloyBytes::from(tx.data)),
                data: None,
            },
            gas: Some(DEFAULT_SWAP_GAS),
            chain_id: Some(self.config.chain_id),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            nonce: Some(inventory.nonce + 1),
            ..Default::default()
        };

        Ok(PreparedTransaction { approval, swap })
    }

    /// Entrypoint for executing the orders
    async fn prepare(&self, orders: Vec<ExecutionOrder>, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::debug!("Executing {} orders. Broadcast config: {}", orders.len(), self.config.broadcast_url);
        unsafe {
            std::env::set_var("RPC_URL", self.config.rpc_url.clone());
        }
        let (_, _, chain) = crate::maker::tycho::chain(self.config.network_name.as_str().to_string()).unwrap();
        // --- Prepare the solutions (solutions = trades encoded with Tycho EVM Encoder) ---
        // @dev This await section has to be done outside of the EVMEncoderBuilder for some unknown reaso, compiler error
        let mut solutions = vec![];
        for order in orders.clone() {
            solutions.push(self.solution(order, env.clone()).await);
        }
        let mut transactions = vec![];
        // --- Encode the solutions ---
        let encoder = EVMEncoderBuilder::new().chain(chain).initialize_tycho_router_with_permit2(env.wallet_private_key.clone());
        match encoder {
            Ok(encoder) => match encoder.build() {
                Ok(encoder) => {
                    // for s in solutions.iter() {
                    // tracing::debug!("Solution: {:?}", s);
                    // match encoder.encode_router_calldata(vec![s.clone()]) {
                    match encoder.encode_router_calldata(solutions.clone()) {
                        Ok(encoded) => {
                            // --- Prepare the transactions ---
                            // tracing::debug!("Encoded {} solutions", encoded.len());
                            for i in 0..orders.len() {
                                // Looping = executing multiple trades, potential conflicts
                                // Need to handle inventory, nonce, etc.
                                // For now it doesn't handle that, for testing purposes
                                let order = orders.get(i);
                                let solution = solutions.get(i);
                                let esolution = encoded.get(i);
                                match (order, solution, esolution) {
                                    (Some(order), Some(solution), Some(esolution)) => match self.encode(solution.clone(), esolution.clone(), context.clone(), inventory.clone(), env.clone()) {
                                        Ok(prepared) => {
                                            transactions.push(prepared);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to prepare transaction: {:?}", e);
                                        }
                                    },
                                    _ => {
                                        tracing::warn!("Order, solution or encoded_solution is None");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to encode router calldata: {:?}", e);
                        }
                    }
                    // }
                }
                Err(e) => {
                    tracing::error!("Failed to build EVMEncoder #2: {:?}", e);
                }
            },
            Err(e) => {
                tracing::error!("Failed to build EVMEncoder #1: {:?}", e);
            }
        };
        transactions
    }

    /// Simulate the transactions, depending on the execution strategy
    async fn simulate(&self, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        self.execution.simulate(self.config.clone(), transactions, env).await
    }

    /// Broadcast the transaction to the network. Swap are sensitive to MEV so we need to be careful
    async fn execute(&self, prepared: Vec<PreparedTransaction>, env: EnvConfig) {
        tracing::info!("Using execution strategy: {}", self.execution.name());
        let _ = self.execution.execute(self.config.clone(), prepared.clone(), env.clone()).await;
    }

    /// Monitor the ProtocolStreamBuilder for new pairs and updates, evaluate if MM bot has opportunities
    async fn run(&mut self, mtx: SharedTychoStreamState, env: EnvConfig) {
        loop {
            tracing::debug!("Connecting ProtocolStreamBuilder for {}", self.config.network_name.as_str().to_string());
            let psbc = PsbConfig {
                filter: ComponentFilter::with_tvl_range(ADD_TVL_THRESHOLD, ADD_TVL_THRESHOLD),
            };
            let state = mtx.read().await;
            let atks = state.atks.clone();
            drop(state);
            let mut components = vec![];
            let mut previous_reference_price = 0.0;
            let mut protosims: HashMap<String, Box<dyn ProtocolSim>> = HashMap::new();
            let psb = crate::maker::tycho::psb(self.config.clone(), env.tycho_api_key.to_string(), psbc.clone(), atks.clone()).await;
            let _stream = match psb.build().await {
                Ok(mut stream) => loop {
                    // Looping
                    match stream.next().await {
                        Some(msg) => match msg {
                            Ok(msg) => {
                                let time = std::time::SystemTime::now();

                                tracing::info!(
                                    "{} '{}' stream: block # {} with {:<2} states updates | Min exec spread: {}", // , + {} pairs, - {} pairs",
                                    self.config.pair_tag,
                                    self.config.network_name.as_str().to_string(),
                                    msg.block_number,
                                    msg.states.len(),
                                    self.config.min_exec_spread_bps,
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
                                    tracing::info!("✅ ProtocolStreamBuilder initialised successfully. Monitoring {} on {} components\n", targets, components.len());
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
                                            let id = cp.id.to_string().to_lowercase();
                                            match protosims.get(&id) {
                                                Some(protosim) => {
                                                    targets.push(ProtoSimComp {
                                                        component: cp.clone(),
                                                        protosim: protosim.clone(),
                                                    });
                                                }
                                                None => {
                                                    tracing::error!("contains: couldn't find protosim for component {}", cp.id);
                                                }
                                            }
                                        }
                                    }

                                    if let Ok(reference_price) = self.fetch_market_price().await {
                                        let cpds = self.prices(&targets);
                                        let identifier = self.identifier.clone();
                                        // --- Price move evaluation ---
                                        let price_move_bps = if previous_reference_price != 0.0 {
                                            ((reference_price - previous_reference_price).abs() / previous_reference_price) * BASIS_POINT_DENO
                                        } else {
                                            // First run - always push to DB since we have no previous price
                                            tracing::info!("First run - always push to DB since we have no previous price");
                                            PRICE_MOVE_THRESHOLD + 1.0
                                        };
                                        let threshold = price_move_bps > PRICE_MOVE_THRESHOLD;
                                        tracing::info!(
                                            "Price movement {} threshold ({} bps), of {:.2} bps, from {} to {}",
                                            if threshold { "above" } else { "below" },
                                            PRICE_MOVE_THRESHOLD,
                                            price_move_bps,
                                            previous_reference_price,
                                            reference_price
                                        );
                                        if threshold {
                                            crate::data::r#pub::prices(NewPricesMessage {
                                                identifier: identifier.clone(),
                                                reference_price,
                                                components: cpds.clone(),
                                                block: msg.block_number,
                                            });
                                            previous_reference_price = reference_price;
                                        } else {
                                            continue;
                                        }
                                        // --- Evaluate ---
                                        let spot_prices = cpds.iter().map(|x| x.price).collect::<Vec<f64>>();
                                        let readjusments = self.evaluate(&targets.clone(), spot_prices.clone(), reference_price);
                                        if !readjusments.is_empty() {
                                            // --- Market context --- Need ALL components and thus all the protosims too
                                            match self.fetch_market_context(components.clone(), &protosims, atks.clone()).await {
                                                Some(context) => {
                                                    context.print();
                                                    // This async block should be optimised as much as possible
                                                    match self.fetch_inventory(env.clone()).await {
                                                        Ok(inventory) => {
                                                            // let context = self.market_context().await;
                                                            let elapsed = time.elapsed().unwrap_or_default().as_millis();
                                                            let orders = self.readjust(context.clone(), inventory.clone(), readjusments, env.clone()).await;
                                                            tracing::info!("Elapsed from block_update to readjustments: {} ms", elapsed);
                                                            if orders.is_empty() {
                                                                // tracing::debug!("No readjustments to execute");
                                                            } else {
                                                                let transactions = self.prepare(orders, context.clone(), inventory.clone(), env.clone()).await;
                                                                // tracing::info!("Publishing trade event for {}", self.config.identifier());
                                                                let executed = self.execution.execute(self.config.clone(), transactions, env.clone()).await;
                                                                tracing::info!("Elapsed from block update to execution: {} ms", elapsed);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::warn!("Failed to get inventory: {:?}", e);
                                                            continue;
                                                        }
                                                    }
                                                }
                                                None => {
                                                    tracing::warn!("Failed to get market context");
                                                }
                                            }
                                        } else {
                                            tracing::debug!("   - No readjustments found");
                                        }
                                    } else {
                                        tracing::error!("Failed to fetch market price");
                                        continue;
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
                    tracing::warn!("Failed to build stream on {}: {:?}. Exiting.", self.config.network_name.as_str().to_string(), e.to_string());
                    return;
                }
            };
            // End
        }
    }
}
