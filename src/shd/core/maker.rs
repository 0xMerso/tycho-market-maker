use std::{collections::HashMap, str::FromStr};

use crate::{
    helpers::global::{cpname, get_alloy_chain, get_component_balances},
    types::{
        config::EnvConfig,
        maker::{CompReadjustment, ExecutionOrder, IMarketMaker, Inventory, MarketContext, MarketMaker, PreparedTransaction, SwapCalculation, TradeDirection},
        tycho::{ProtoSimComp, PsbConfig, SharedTychoStreamState},
    },
    utils::r#static::{ADD_TVL_THRESHOLD, APPROVE_FN_SIGNATURE, BASIS_POINT_DENO, DEFAULT_APPROVE_GAS, DEFAULT_SWAP_GAS, NULL_ADDRESS, SHARE_POOL_BAL_SWAP_BPS},
};
use alloy::{
    consensus::transaction,
    providers::{Provider, ProviderBuilder},
    rpc::types::{
        TransactionInput, TransactionRequest,
        simulate::{SimBlock, SimulatePayload},
    },
    signers::local::PrivateKeySigner,
    sol_types::SolValue,
};

use alloy_primitives::{Address, B256, U256};
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

use super::pricefeed::chainlink;
use alloy_primitives::Bytes as AlloyBytes;

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
    fn spot_prices(&self, psc: &Vec<ProtoSimComp>) -> Vec<f64> {
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
                    ss.push(price);
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
    /// ! Compute base/USD and quote/USD, based on a arbitrary path ! Just a valid path !
    async fn fetch_market_context(&self, components: Vec<ProtocolComponent>, protosims: &HashMap<std::string::String, Box<dyn ProtocolSim>>, tokens: Vec<Token>) -> Option<MarketContext> {
        let time = std::time::SystemTime::now();
        match crate::utils::evm::eip1559_fees(self.config.rpc.clone()).await {
            Ok(eip1559_fees) => {
                let native_gas_price = crate::utils::evm::gas_price(self.config.rpc.clone()).await;
                let eth_to_usd = self.fetch_eth_usd().await;
                let provider = ProviderBuilder::new().on_http(self.config.rpc.clone().parse().unwrap());
                let block: alloy::rpc::types::Block = provider.get_block_by_number(alloy::eips::BlockNumberOrTag::Latest, false).await.unwrap().unwrap();
                let base_to_eth_vp = super::routing::find_path(components.clone(), self.base.address.to_string().to_lowercase(), self.config.gas_token.to_lowercase());
                let quote_to_eth_vp = super::routing::find_path(components.clone(), self.quote.address.to_string().to_lowercase(), self.config.gas_token.to_lowercase());
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
                        let base_to_eth = super::routing::quote(to_eth_ptss.clone(), tokens.clone(), base_to_eth_vp.token_path.clone());
                        let quote_to_eth = super::routing::quote(to_eth_ptss.clone(), tokens.clone(), quote_to_eth_vp.token_path.clone());
                        // tracing::debug!("Gas: {:?} | Native: {}", eip1559_fees, native_gas_price);
                        let elasped = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!(" - Market context fetched in {} ms", elasped);
                        match (base_to_eth, quote_to_eth) {
                            (Some(base_to_eth), Some(quote_to_eth)) => Some(MarketContext {
                                base_to_eth,
                                quote_to_eth,
                                eth_to_usd,
                                max_fee_per_gas: eip1559_fees.max_fee_per_gas,
                                max_priority_fee_per_gas: eip1559_fees.max_priority_fee_per_gas,
                                native_gas_price,
                                block,
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
            Err(e) => {
                tracing::error!("Failed to fetch EIP-1559 fees: {:?}", e);
                return None;
            }
        }
    }

    // Evaluate if given pools are out of range (= require intervention)
    // Targets are the pools to monitor, nothing more
    async fn evaluate(&self, targets: &Vec<ProtoSimComp>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment> {
        let mut orders = vec![];
        if sps.is_empty() || targets.len() != sps.len() {
            tracing::warn!("Components targets and spot prices length mismatch ({} != {})", targets.len(), sps.len());
            return vec![];
        }
        // tracing::debug!("Evaluating {} pools...", targets.len());
        for (i, psc) in targets.iter().enumerate() {
            let spot = sps[i];
            let spread = spot - reference;
            let spread_bps = spread / reference * BASIS_POINT_DENO;
            // Check if the spread is above the threshold
            if spread_bps.abs() > self.config.spread as f64 {
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
    /// - How to allocate the size of each readjustment, they are dependent on each other
    /// "Optimal swap is to swap until marginal price + fee = market price"
    async fn readjust(&self, context: MarketContext, inventory: Inventory, mut adjustments: Vec<CompReadjustment>, env: EnvConfig) -> Vec<ExecutionOrder> {
        // --- Ordering ---
        // Order by spread (absolute value)
        adjustments.sort_by(|a, b| {
            if a.spread_bps > b.spread_bps {
                std::cmp::Ordering::Greater
            } else if a.spread_bps < b.spread_bps {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });
        let mut orders = vec![];
        // tracing::debug!("Profitability evaluation: {}", self.config.profitability);
        for adjustment in adjustments.iter() {
            match get_component_balances(self.config.clone(), adjustment.psc.component.clone(), env.tycho_api_key.clone()).await {
                Some(balances) => {
                    // for b in balances.iter() {
                    //     tracing::debug!(" - Attribute: {}", b.0);
                    // }
                    // --- Token & Amounts ---
                    let buying = adjustment.buying.clone();
                    let buying_pow = 10f64.powi(buying.decimals as i32);
                    let pool_buying_balance = balances.get(&buying.address.to_string().to_lowercase()).unwrap_or_else(|| panic!("Failed to get buying balance"));
                    let pool_buying_balance_divided = (*pool_buying_balance as f64) / buying_pow;
                    if pool_buying_balance_divided < f64::EPSILON {
                        tracing::info!("pool_buying_balance_divided < 0 !");
                    }
                    let selling = adjustment.selling.clone();
                    let selling_pow = 10f64.powi(selling.decimals as i32);
                    let pool_selling_balance = balances.get(&selling.address.to_string().to_lowercase()).unwrap_or_else(|| panic!("Failed to get selling balance"));
                    let pool_selling_balance_divided = (*pool_selling_balance as f64) / selling_pow;
                    if pool_selling_balance_divided < f64::EPSILON {
                        tracing::warn!("Cannot readjust, skipping due to pool_selling_balance_divided < 0 !");
                        continue;
                    }
                    let base_to_quote = selling == self.base; // ! Key
                    // --- Size & Allocation --- v2
                    // let depths = self.config.depths.clone();
                    // for depth in depths {}
                    // --- Size & Allocation --- v1
                    let inventory_balance = if base_to_quote { inventory.base_balance } else { inventory.quote_balance };
                    let inventory_balance_divided = (inventory_balance as f64) / selling_pow;
                    // Percentage of the pool balance
                    let optimal = pool_selling_balance_divided * SHARE_POOL_BAL_SWAP_BPS / BASIS_POINT_DENO;
                    // Sample depth
                    let max_alloc = inventory_balance_divided * self.config.max_trade_allocation; // Capping the allocation to a maximum
                    // let selling_amount = inventory_balance_divided * self.config.max_trade_allocation;
                    // ! ------------- Tmp -------------
                    let selling_amount = optimal;
                    // -------------
                    let buying_amount = if base_to_quote { selling_amount * adjustment.spot } else { selling_amount / adjustment.spot };
                    // --- Debug ---
                    let pool_msg = format!(
                        " - Pool {} | Tycho Spot: {:>12.5} vs ref {:>12.5} | Spread: {:>7.2} {} = {:>5.0} bps)",
                        cpname(adjustment.psc.component.clone()),
                        adjustment.spot,
                        adjustment.reference,
                        adjustment.spread,
                        self.quote.symbol,
                        adjustment.spread_bps,
                    );
                    let inventory_msg = format!(
                        " Inventory: {:.2} {} | Optimal: {:.} | Max: {:.5} | Selling {:.5} {} for {:.5} {}",
                        inventory_balance_divided, selling.symbol, optimal, max_alloc, selling_amount, selling.symbol, buying_amount, buying.symbol
                    );
                    tracing::debug!("{} | {}", pool_msg, inventory_msg);
                    // --- Prepa Exec ---
                    let powered_selling_amount = selling_amount * selling_pow;
                    let powered_selling_amount_bg = BigUint::from(powered_selling_amount.floor() as u128);
                    let powered_buying_amount = buying_amount * buying_pow;
                    // --- Allocation valorisation with market context ---
                    let (selling_amount_worth_eth, buying_amount_worth_eth) = match base_to_quote {
                        true => (selling_amount * context.base_to_eth, buying_amount * context.quote_to_eth), // For 1 unit of selling/buying token !
                        false => (selling_amount * context.quote_to_eth, buying_amount * context.base_to_eth), // For 1 unit of selling/buying token !
                    };
                    let (selling_amount_worth_usd, buying_amount_worth_usd) = (selling_amount_worth_eth * context.eth_to_usd, buying_amount_worth_eth * context.eth_to_usd);
                    // tracing::info!(
                    //     " - selling_amount_worth_eth: {} $ETH | buying_amount_worth_eth: {} $ETH ",
                    //     selling_amount_worth_eth,
                    //     buying_amount_worth_eth
                    // );
                    // tracing::info!(
                    //     " - selling_amount_worth_usd: {} $    | buying_amount_worth_usd: {} $",
                    //     selling_amount_worth_usd,
                    //     buying_amount_worth_usd
                    // );

                    // --- Simulation ---
                    // ? Should be done with x amounts, to pick the best one
                    // See sampdepths
                    match adjustment.psc.protosim.get_amount_out(powered_selling_amount_bg.clone(), &selling, &buying) {
                        Ok(result) => {
                            // --- Price Impact & Gas Fees ---
                            let amount_out_powered = result.amount.to_f64().unwrap_or(0.0);
                            let amount_out_divided = amount_out_powered / 10f64.powi(buying.decimals as i32); // [new]

                            let slippage_bps = self.config.slippage * BASIS_POINT_DENO;
                            let amount_out_min_divided = amount_out_divided * (BASIS_POINT_DENO - slippage_bps) / BASIS_POINT_DENO;
                            let amount_out_min_powered = amount_out_min_divided * buying_pow;

                            let gas_units = result.gas.to_string().parse::<u128>().unwrap_or_default();
                            let gas_cost_eth = (gas_units.saturating_mul(context.native_gas_price)) as f64 / 1e18; // Gwei 10^9 + Gwei 10^9 = 10^18
                            let gas_cost_usd = gas_cost_eth * context.eth_to_usd;
                            let gas_cost_in_output = match base_to_quote {
                                true => gas_cost_eth / context.quote_to_eth,
                                false => gas_cost_eth / context.base_to_eth,
                            };
                            tracing::debug!(
                                " - Simulation: {} {} for {} {} | Gas cost : {:.5} $ | Gas cost in output: {:.2} %",
                                selling_amount,
                                selling.symbol,
                                amount_out_divided,
                                buying.symbol,
                                gas_cost_usd,
                                gas_cost_in_output * 100.0
                            );
                            // --- Swap costs --- LP Fee + Price impact
                            let average_sell_price = match base_to_quote {
                                true => amount_out_divided / selling_amount,
                                false => 1. / (amount_out_divided / selling_amount),
                            };
                            let delta = average_sell_price - adjustment.spot;
                            let price_impact_bps = ((delta / adjustment.spot) * BASIS_POINT_DENO).round();
                            // --- Swap costs --- Gas
                            let average_sell_price_net_gas = match base_to_quote {
                                true => (amount_out_divided - gas_cost_in_output) / selling_amount,
                                false => 1. / ((amount_out_divided - gas_cost_in_output) / selling_amount),
                            };
                            let delta_net_of_gas = average_sell_price_net_gas - adjustment.spot;
                            let price_impact_net_of_gas_bps = ((delta_net_of_gas / adjustment.spot) * BASIS_POINT_DENO).round(); // Potential execution price, if no slippage
                            // ? Make the disctinction between price impact and pool fee | Fee = amount * pool_fee | Price impact = (amount * pool_fee) - amount_out
                            // tracing::debug!(
                            //     " - base_to_quote: {} | swap cost (LP/PI): {} (bps) | gas_cost_usd: {:.4}$ | Average sell price: {:.4} (spot = {}) | Delta: {:.4}",
                            //     base_to_quote,
                            //     price_impact_bps,
                            //     gas_cost_usd,
                            //     average_sell_price,
                            //     adjustment.spot,
                            //     delta
                            // );

                            // tracing::debug!(
                            //     " - Price impact net of gas: {} (bps) | Average sell price net of gas: {:.4} | Delta net of gas: {:.4}",
                            //     price_impact_net_of_gas_bps,
                            //     average_sell_price_net_gas,
                            //     delta_net_of_gas
                            // );
                            let potential_profit_delta = match base_to_quote {
                                true => average_sell_price_net_gas - adjustment.reference,
                                false => adjustment.reference - average_sell_price_net_gas,
                            };
                            let potential_profit_delta_spread_bps = potential_profit_delta / adjustment.reference * BASIS_POINT_DENO;
                            let potential_profit_delta_spread_bps_abs = potential_profit_delta_spread_bps; //.abs(); // ! Tmp abs()
                            let profitable = potential_profit_delta_spread_bps_abs > self.config.min_exec_spread;
                            tracing::debug!(
                                "   ---> Profit: {} | average_sell_price_net_gas: {:.4} vs reference_price: {:.4} | potential_profit_delta: {:.5} | potential_profit_delta_spread_bps: {:.2}",
                                if potential_profit_delta > 0. { "🟢" } else { "🟠" },
                                average_sell_price_net_gas,
                                adjustment.reference,
                                potential_profit_delta,
                                potential_profit_delta_spread_bps
                            );
                            if profitable {
                                // Compensation -> Skipped
                                // --- Prepa execution ---
                                let calculation = SwapCalculation {
                                    base_to_quote,
                                    selling_amount,
                                    buying_amount,
                                    powered_selling_amount,
                                    powered_buying_amount,
                                    // Post-swap
                                    amount_out_divided,
                                    amount_out_powered,
                                    amount_out_min_divided,
                                    amount_out_min_powered,
                                    gas_units,
                                    // Misc
                                    average_sell_price,
                                    average_sell_price_net_gas,
                                    gas_cost_eth,
                                    gas_cost_usd,
                                    gas_cost_in_output_token: gas_cost_in_output,
                                    selling_worth_usd: selling_amount_worth_usd,
                                    buying_worth_usd: buying_amount_worth_usd,
                                    profit_delta_bps: potential_profit_delta_spread_bps_abs,
                                    profitable,
                                };
                                let order = ExecutionOrder {
                                    adjustment: adjustment.clone(),
                                    calculation,
                                };
                                orders.push(order);
                                tracing::debug!(" ----------------- New order pushed -----------------");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to simulate get amount out: {:?}", e);
                            continue;
                        }
                    }
                }
                None => {
                    tracing::warn!("Failed to get component balances");
                }
            }
        }

        // Make sure no conflict between readjustments
        // Make sure we don't run out of gas by keeping a minimum post-swap balance to 0.01 ETH
        // if !self.config.profitability {}
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
            " - {} : Building Tycho solution: Buying {} with {} | Amount in: {} | Amount out: {} | Amount out min: {}",
            cpname(order.adjustment.psc.component.clone()),
            order.adjustment.buying.symbol,
            order.adjustment.selling.symbol,
            amount_in,
            amount_out,
            amount_out_min
        );
        let swap = tycho_execution::encoding::models::Swap::new(order.adjustment.psc.component.clone(), input.clone(), output.clone(), split);
        // tracing::debug!(" - Swap: {:?}", swap);
        // Swap { component: ProtocolComponent { id: "88e6a0c2ddd26feeb64f039a2c41296fcb3f5640", protocol_system: "uniswap_v3", protocol_type_name: "uniswap_v3_pool", chain: Ethereum, tokens: [Bytes(0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48), Byte (0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2)], contract_addresses: [], static_attributes: {"tick_spacing": Bytes(0x0a), "fee": Bytes(0x01f4), "pool_address": Bytes(0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640)}, change: Update, creation_tx: Bytes(0x125e0b641d4a4b08806bf52c0c6757648c9963bcda8681e4f996f09e00d4c2cc), created_at: 2021-05-05T21:42:11 }, token_in: Bytes(0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2), token_out: Bytes(0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48), split: 0.0
        Solution {
            // Addresses
            sender: tycho_simulation::tycho_core::Bytes::from_str(env.wallet_public_key.to_lowercase().as_str()).unwrap(),
            receiver: tycho_simulation::tycho_core::Bytes::from_str(env.wallet_public_key.to_lowercase().as_str()).unwrap(),
            given_token: input.clone(),
            checked_token: output.clone(),
            // Others fields
            given_amount: amount_in.clone(),
            slippage: Some(self.config.slippage as f64), // Slippage in decimal < 1, because 1.0 = 100%
            exact_out: false,                            // It's an exact in solution
            expected_amount: Some(amount_out),
            checked_amount: Some(amount_out_min), // The amount out will not be checked in execution
            swaps: vec![swap.clone()],
            ..Default::default()
        }
    }

    /// Convert a solution to a transaction payload
    /// Also build the approval transaction, presumed needed (never infinite approval)
    fn encode(&self, solution: Solution, tx: Transaction, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Result<PreparedTransaction, String> {
        let max_priority_fee_per_gas = context.max_priority_fee_per_gas; // 1 Gwei, not suited for L2s.
        let max_fee_per_gas = context.max_fee_per_gas;

        // 1. Approvals (Tycho router) with Permit2
        let amount: u128 = solution.given_amount.clone().to_string().parse().expect("Couldn't convert given_amount to u128"); // ?
        let args = (Address::from_str(&self.config.permit2).expect("Couldn't convert permit2 to address"), amount);
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
            chain_id: Some(self.config.chainid),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            nonce: Some(inventory.nonce),
            ..Default::default()
        };

        // 2. Swap --- No bribe for now ---
        let swap = TransactionRequest {
            to: Some(alloy_primitives::TxKind::Call(Address::from_slice(&tx.to))),
            from: Some(env.wallet_public_key.parse().expect("Failed to parse wallet public key")),
            value: Some(U256::from(0)),
            input: TransactionInput {
                input: Some(AlloyBytes::from(tx.data)),
                data: None,
            },
            gas: Some(DEFAULT_SWAP_GAS),
            chain_id: Some(self.config.chainid),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            nonce: Some(inventory.nonce + 1),
            ..Default::default()
        };

        Ok(PreparedTransaction { approval, swap })
    }

    /// Entrypoint for executing the orders
    async fn prepare(&self, orders: Vec<ExecutionOrder>, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::debug!("Executing {} orders. Broadcast config: {}", orders.len(), self.config.broadcast);
        unsafe {
            std::env::set_var("RPC_URL", self.config.rpc.clone());
        }
        let (_, _, chain) = crate::helpers::global::chain(self.config.network.clone()).unwrap();
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
                            tracing::debug!("Encoded {} solutions", encoded.len());
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
                                            tracing::debug!("Prepared transaction #{}: Approval to {} | Swap to {}", i + 1, solution.given_token, solution.checked_token);
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

    /// No interdependencies between orders, so we can simulate them all at once
    /// In a recursive or dependent way, we would need to simulate each order one by one, possible with state overwrite
    async fn simulate(&self, transactions: Vec<PreparedTransaction>, env: EnvConfig) {
        let alloy_chain = get_alloy_chain(self.config.network.clone()).expect("Failed to get alloy chain");
        let rpc = self.config.rpc.parse::<url::Url>().unwrap().clone(); // ! Custom per network
        let pk = env.wallet_private_key.clone();
        let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
        let signer = alloy::network::EthereumWallet::from(wallet.clone());
        let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc);
        // Flashbot Bundle simu, no need for pure EVM simulation
        let mut transactions = transactions.clone();
        transactions.retain(|t| {
            let sender = t.approval.from.unwrap_or_default().to_string().to_lowercase();
            let matching = wallet.address().to_string().eq_ignore_ascii_case(sender.clone().as_str());
            !matching
        });
        // Using only the first transaction for now
        tracing::debug!("Simulating {} transactions (keeping only the first for now)", transactions.len());
        let first = transactions.first().expect("No transactions found");
        // Bundle simulation
        let calls = vec![first.approval.clone(), first.swap.clone()];
        let payload = SimulatePayload {
            block_state_calls: vec![SimBlock {
                block_overrides: None,
                state_overrides: None,
                calls,
            }],
            trace_transfers: true,
            validation: true,
            return_full_transactions: true,
        };
    }

    /// Broadcast the transaction to the network
    /// Swap are sensitive to MEV so we need to be careful
    async fn broadcast(&self) {
        tracing::debug!("Broadcasting");
    }

    /// Monitor the ProtocolStreamBuilder for new pairs and updates, evaluate if MM bot has opportunities
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
                                    " 💎 '{}' stream: block # {} with {:<2} states updates | Market price: {}", // , + {} pairs, - {} pairs",
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
                                    tracing::info!(" ✅ ProtocolStreamBuilder initialised successfully. Monitoring {} on {} components", targets, components.len());
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

                                    // --- Evaluate ---
                                    let prices = self.spot_prices(&targets);
                                    let readjusments = self.evaluate(&targets.clone(), prices.clone(), reference).await;
                                    if !readjusments.is_empty() {
                                        // --- Market context --- Need ALL components and thus all the protosims too
                                        match self.fetch_market_context(components.clone(), &protosims, atks.clone()).await {
                                            Some(context) => {
                                                context.print();
                                                // This async block should be optimised as much as possible
                                                match self.fetch_inventory(env.clone()).await {
                                                    Ok(inventory) => {
                                                        // let context = self.market_context().await;
                                                        let _elasped = time.elapsed().unwrap_or_default().as_millis();
                                                        // tracing::info!(" - Evaluation until readjustments took {} ms", elasped);
                                                        let orders = self.readjust(context.clone(), inventory.clone(), readjusments, env.clone()).await;
                                                        if orders.is_empty() {
                                                            // tracing::debug!("No readjustments to execute");
                                                        } else {
                                                            self.prepare(orders, context.clone(), inventory.clone(), env.clone()).await;
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
