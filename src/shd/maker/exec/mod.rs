use async_trait::async_trait;
use std::collections::HashMap;
use std::result::Result;
use std::str::FromStr;

use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::simulate::{SimBlock, SimulatePayload},
    signers::local::PrivateKeySigner,
};
use alloy_primitives::B256;

use crate::{
    maker::tycho::get_alloy_chain,
    types::{
        config::{EnvConfig, MarketMakerConfig, NetworkName},
        maker::{BroadcastData, SimulatedData, Trade, TradeStatus},
        moni::NewTradeMessage,
    },
};

pub mod chain;

/// Execution strategy names
#[derive(Debug, Clone, PartialEq)]
pub enum ExecStrategyName {
    MainnetStrategy,
    BaseStrategy,
    UnichainStrategy,
}

impl ExecStrategyName {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecStrategyName::MainnetStrategy => "Mainnet_Strategy",
            ExecStrategyName::BaseStrategy => "Base_Strategy",
            ExecStrategyName::UnichainStrategy => "Unichain_Strategy",
        }
    }
}

/// Dynamic execution strategy factory
pub struct ExecStrategyFactory;

impl ExecStrategyFactory {
    /// Create the appropriate execution strategy based on broadcast URL configuration
    pub fn create(network: &str) -> Box<dyn ExecStrategy> {
        match NetworkName::from_str(network) {
            Some(NetworkName::Ethereum) => Box::new(chain::mainnet::MainnetExec::new()),
            Some(NetworkName::Base) => Box::new(chain::base::BaseExec::new()),
            Some(NetworkName::Unichain) => Box::new(chain::unichain::UnichainExec::new()),
            None => panic!("Unknown network '{}', please check the network name in the config file", network),
        }
    }
}

/// Execution strategy trait for handling different execution methods
#[async_trait]
pub trait ExecStrategy: Send + Sync {
    /// Get the strategy name for logging
    fn name(&self) -> String;

    /// Pre-execution hook
    async fn pre_hook(&self, _config: &MarketMakerConfig) {
        tracing::info!("[{}] default_pre_exec_hook", self.name());
    }

    /// Post-execution hook
    async fn post_hook(&self, config: &MarketMakerConfig, trades: Vec<Trade>, identifier: String) {
        tracing::info!("{}: default_post_exec_hook", self.name());
        tracing::info!("Saving trades for instance identifier: {}", identifier);
        if config.publish_events {
            for trade in trades {
                if trade.metadata.status != TradeStatus::BroadcastSucceeded {
                    tracing::error!("Trade not broadcasted, skipping post-exec hook");
                    continue;
                } else {
                    let _ = crate::data::r#pub::trade(NewTradeMessage {
                        identifier: identifier.clone(), // Use passed identifier for trade tracking
                        data: trade.metadata.clone(),
                    });
                }
            }
        }
    }

    /// Execute the prepared transactions (orchestration)
    async fn execute(&self, config: MarketMakerConfig, _trades: Vec<Trade>, env: EnvConfig, identifier: String) -> Result<Vec<Trade>, String> {
        self.pre_hook(&config).await;
        tracing::info!("[{}] Executing {} trades", self.name(), _trades.len());
        let mut trades = _trades.clone();
        let mut trades_with_simu = if config.skip_simulation {
            tracing::info!("🚀 Skipping simulation - direct execution enabled");
            _trades
        } else {
            let smd = self.simulate(config.clone(), _trades.clone(), env.clone()).await?;
            for (x, smd) in smd.iter().enumerate() {
                trades[x].metadata.simulation = Some(smd.clone());
            }
            trades
        };

        // Set status to SimulationSucceeded for all trades
        for trade in trades_with_simu.iter_mut() {
            trade.metadata.status = TradeStatus::SimulationSucceeded;
        }

        let bd = self.broadcast(trades_with_simu.clone(), config.clone(), env).await?;
        for (x, bd) in bd.iter().enumerate() {
            trades_with_simu[x].metadata.broadcast = Some(bd.clone());
        }

        // Set status to SimulationSucceeded for all trades
        for trade in trades_with_simu.iter_mut() {
            trade.metadata.status = TradeStatus::BroadcastSucceeded;
        }

        self.post_hook(&config, trades_with_simu.clone(), identifier).await;
        Ok(trades_with_simu)
    }

    /// Simulate transactions to validate they will succeed before execution
    /// Pure EVM simulation, no bundle, etc.
    async fn simulate(&self, config: MarketMakerConfig, trades: Vec<Trade>, env: EnvConfig) -> Result<Vec<SimulatedData>, String> {
        tracing::info!("{}: Simulating {} trades", self.name(), trades.len());
        let initial = trades.len();
        // tracing::debug!("default_simulate: {} trades", trades.len());
        let alloy_chain = get_alloy_chain(config.network_name.as_str().to_string()).expect("Failed to get alloy chain");
        let rpc = config.rpc_url.parse::<url::Url>().unwrap().clone(); // ! Custom per network
        let pk = env.wallet_private_key.clone();
        let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
        tracing::debug!("Wallet configured: {:?}", wallet.address().to_string().to_lowercase());
        let signer = alloy::network::EthereumWallet::from(wallet.clone());

        // --- Filtering trades ---
        let mut trades = trades.clone();
        trades.retain(|t| {
            let sender = if let Some(approve) = &t.approve {
                approve.from.unwrap_or_default().to_string().to_lowercase()
            } else {
                t.swap.from.unwrap_or_default().to_string().to_lowercase()
            };
            wallet.address().to_string().eq_ignore_ascii_case(sender.clone().as_str())
        });
        let removed = initial - trades.len();
        if removed > 0 {
            tracing::debug!("Removed {} trades (criterias: not owned by the wallet)", removed);
        }
        let mut simulations = HashMap::new();
        for i in 0..trades.len() {
            simulations.insert(i, false);
        }
        let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());
        let mut output = vec![];

        for (_x, tx) in trades.iter().enumerate() {
            let time = std::time::Instant::now();
            let mut calls = vec![];
            if let Some(approval) = &tx.approve {
                calls.push(approval.clone());
            }
            calls.push(tx.swap.clone());
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
            let mut smd = SimulatedData::default();
            match provider.simulate(&payload).await {
                Ok(output) => {
                    for block in output.iter() {
                        tracing::trace!("🔮 Simulated on block #{} ...", block.inner.header.number);

                        match block.calls.len() {
                            1 => {
                                // No approval needed, only swap
                                let swap = &block.calls[0];
                                let took = time.elapsed().as_millis();
                                let now = std::time::Instant::now().elapsed().as_millis();
                                smd.simulated_at_ms = now;
                                smd.simulated_took_ms = took;
                                smd.estimated_gas = swap.gas_used as u128;
                                smd.status = swap.status;
                                smd.error = None;

                                if !swap.status {
                                    let reason = swap.error.clone().unwrap().message;
                                    tracing::error!(" - Simulation failed on swap call. No broadcast. Reason: {}", reason);
                                    smd.error = Some(reason);
                                }
                            }
                            2 => {
                                // First call is approval, second is swap
                                let approval = &block.calls[0]; // Approval is ignored for now
                                let swap = &block.calls[1];

                                tracing::trace!(" - Approval simulation: Gas: {} | Status: {}", approval.gas_used, approval.status);

                                let took = time.elapsed().as_millis();
                                let now = std::time::Instant::now().elapsed().as_millis();
                                smd.simulated_at_ms = now;
                                smd.simulated_took_ms = took;
                                smd.estimated_gas = swap.gas_used as u128;
                                smd.status = swap.status;
                                smd.error = None;

                                if !swap.status {
                                    let reason = swap.error.clone().unwrap().message;
                                    tracing::error!(" - Simulation failed on swap call. No broadcast. Reason: {}", reason);
                                    smd.error = Some(reason);
                                }
                            }
                            _ => {
                                tracing::error!("Invalid number of calls in simulation: {}", block.calls.len());
                                smd.status = false;
                                smd.error = Some(format!("Invalid number of calls: {}", block.calls.len()));
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to simulate: {:?}", e);
                    smd.status = false;
                    smd.error = Some(format!("Simulation error: {:?}", e));
                }
            };
            output.push(smd);
        }
        Ok(output)
    }

    /// Broadcast transactions (execution)
    async fn broadcast(&self, prepared: Vec<Trade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<BroadcastData>, String> {
        tracing::info!("{}: Broadcasting {} trades", self.name(), prepared.len());
        let alloy_chain = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
        let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
        let pk = env.wallet_private_key.clone();
        let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
        let signer = alloy::network::EthereumWallet::from(wallet.clone());
        let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());

        if env.testing {
            tracing::info!("Skipping broadcast ! Testing mode enabled");
            return Ok(Vec::new());
        }

        let mut output = Vec::new();
        for (x, tx) in prepared.iter().enumerate() {
            tracing::debug!(" - Tx: #{} | Broadcasting on {}", x, mmc.network_name.as_str().to_string());
            if tx.metadata.simulation.is_some() && tx.metadata.simulation.as_ref().unwrap().status == false {
                tracing::error!("Simulation failed for tx: #{}, no broadcast", x);
                continue;
            }
            let time = std::time::SystemTime::now();
            let mut bd = BroadcastData::default();

            // Handle optional approval transaction
            let _approval_result = if let Some(approval_tx) = &tx.approve {
                match provider.send_transaction(approval_tx.clone()).await {
                    Ok(approve) => {
                        let took = time.elapsed().unwrap_or_default().as_millis() as u128;
                        tracing::debug!("   - Explorer: {}tx/{} | Approval shoot took {} ms", mmc.explorer_url, approve.tx_hash(), took);
                        Some(approve)
                    }
                    Err(e) => {
                        tracing::error!("Failed to send approval transaction: {:?}", e);
                        None
                    }
                }
            } else {
                tracing::debug!("   - Skipping approval transaction (infinite_approval enabled)");
                None
            };

            // Send swap transaction
            let broadcasted = std::time::Instant::now().elapsed().as_millis();
            match provider.send_transaction(tx.swap.clone()).await {
                Ok(swap) => {
                    let took = time.elapsed().unwrap_or_default().as_millis() as u128;
                    let tx_description = if tx.approve.is_some() { "Swap (+ approval)" } else { "Swap only" };
                    tracing::debug!("   - Explorer: {}tx/{} | {} shoot took {} ms", mmc.explorer_url, swap.tx_hash(), tx_description, took);
                    bd.broadcasted_at_ms = broadcasted;
                    bd.broadcasted_took_ms = took;
                    bd.hash = swap.tx_hash().to_string();
                }
                Err(e) => {
                    tracing::error!("Failed to send swap transaction: {:?}", e);
                }
            }
            output.push(bd);
        }
        Ok(output)
    }
}
