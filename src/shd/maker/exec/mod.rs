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
        maker::{ExecutedPayload, PreparedTrade, SimulatedData},
    },
    utils::constants::HAS_EXECUTED,
};

/// Execution strategy trait for handling different execution methods
#[async_trait]
pub trait ExecStrategy: Send + Sync {
    /// Get the strategy name for logging
    fn name(&self) -> &'static str;

    /// Pre-execution hook
    async fn pre_exec_hook(&self, config: &MarketMakerConfig);

    /// Post-execution hook
    async fn post_exec_hook(&self, config: &MarketMakerConfig, transactions: Vec<ExecutedPayload>, identifier: String);

    /// Execute the prepared transactions (orchestration)
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTrade>, env: EnvConfig, identifier: String) -> Result<Vec<ExecutedPayload>, String>;

    /// Simulate transactions (validation)
    async fn simulate(&self, config: MarketMakerConfig, transactions: Vec<PreparedTrade>, env: EnvConfig) -> Result<Vec<PreparedTrade>, String>;

    /// Broadcast transactions (execution)
    async fn broadcast(&self, prepared: Vec<PreparedTrade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<ExecutedPayload>, String>;
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

// ! Put that into default trait class ?

/// Simulate transactions to validate they will succeed before execution
/// Pure EVM simulation, no bundle, etc.
pub async fn default_simulate(transactions: Vec<PreparedTrade>, config: &MarketMakerConfig, env: EnvConfig) -> Vec<PreparedTrade> {
    let initial_len = transactions.len();
    tracing::debug!("default_simulate: {} transactions", transactions.len());
    let alloy_chain = get_alloy_chain(config.network_name.as_str().to_string()).expect("Failed to get alloy chain");
    let rpc = config.rpc_url.parse::<url::Url>().unwrap().clone(); // ! Custom per network
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    tracing::debug!("Wallet configured: {:?}", wallet.address().to_string().to_lowercase());
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let mut transactions = transactions.clone();
    transactions.retain(|t| {
        let sender = t.approve.from.unwrap_or_default().to_string().to_lowercase();
        wallet.address().to_string().eq_ignore_ascii_case(sender.clone().as_str())
    });
    let removed = initial_len - transactions.len();
    if removed > 0 {
        tracing::debug!("Removed {} transactions (criterias: not owned by the wallet)", removed);
    }
    let mut simulations = HashMap::new();
    for i in 0..transactions.len() {
        simulations.insert(i, false);
    }
    let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());
    let mut succeeded = vec![];

    if !transactions.is_empty() {
        for (x, tx) in transactions.iter().enumerate() {
            let time = std::time::Instant::now();
            let calls = vec![tx.approve.clone(), tx.swap.clone()];
            let names = ["approval".to_string(), "swap".to_string()];
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
            match provider.simulate(&payload).await {
                Ok(output) => {
                    for block in output.iter() {
                        tracing::trace!("Simulated 🔮 on block #{} ...", block.inner.header.number);
                        for (x, scr) in block.calls.iter().enumerate() {
                            let name = names.get(x).unwrap();
                            let took = time.elapsed().as_millis();
                            tracing::trace!(" - SimCallResult for '{}': Gas: {} | Simulation status: {} | Took: {} ms", name, scr.gas_used, scr.status, took);

                            let mut simdata = SimulatedData {
                                simulated_at_ms: 0,
                                simulated_took_ms: took,
                                estimated_gas: scr.gas_used as u128,
                                status: scr.status,
                                error: None,
                            };

                            if !scr.status {
                                let reason = scr.error.clone().unwrap().message;
                                tracing::error!(" - Simulation failed on SimCallResult on '{}'. No broadcast. Reason: {}", name, reason);
                                // ? Publish failed status for both transactions ?
                            } else {
                                succeeded.push(tx.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to simulate: {:?}", e);
                }
            };
            simulations.insert(x, true);
        }
    }
    succeeded
}

/// Shared broadcasting function used by all strategies
pub async fn default_broadcast(prepared: Vec<PreparedTrade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<ExecutedPayload>, String> {
    tracing::debug!("default_broadcast: {} transactions", prepared.len());
    let alloy_chain = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
    let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());
    let mut results = Vec::new();

    if env.testing {
        tracing::info!("🧪 Skipping broadcast ! Testing mode enabled");
        return Ok(results);
    }

    for (x, tx) in prepared.iter().enumerate() {
        tracing::debug!("Tx: #{} | Broadcasting on {}", x, mmc.network_name.as_str().to_string());
        // --- Sending, without waiting for receipt ---
        let time = std::time::SystemTime::now();
        let mut exec = ExecutedPayload::default();

        match provider.send_transaction(tx.approve.clone()).await {
            Ok(approve) => {
                let approval_time = time.elapsed().unwrap_or_default().as_millis();
                tracing::debug!("Explorer: {}tx/{} | Approval shoot took {} ms", mmc.explorer_url, approve.tx_hash(), approval_time);
                exec.approval.sent = true;
                exec.approval.hash = approve.tx_hash().to_string();
                HAS_EXECUTED.store(true, std::sync::atomic::Ordering::Relaxed);
                match provider.send_transaction(tx.swap.clone()).await {
                    Ok(swap) => {
                        let swap_time = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Explorer: {}tx/{} | Swap (+ approval) shoot took {} ms", mmc.explorer_url, swap.tx_hash(), swap_time);
                        exec.swap.sent = true;
                        exec.swap.hash = swap.tx_hash().to_string();
                        //  --- Wait for receipt ---
                        let time = std::time::SystemTime::now();

                        let mut broadcast_data = BroadcastData {
                            broadcasted_at_ms: 0,
                            broadcasted_took_ms: swap_time,
                            hash: "".to_string(),
                            broadcast_error: None,
                            receipt: None,
                        };

                        let approve_receipt = approve.get_receipt().await; // ! Optional ?
                        let swap_receipt = swap.get_receipt().await;
                        let total_time = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Approval get_receipt + Swap get_receipt took {} ms", total_time);
                        match (approve_receipt, swap_receipt) {
                            (Ok(approval_receipt), Ok(swap_receipt)) => {
                                tracing::debug!("Approval receipt: status: {:?}", approval_receipt.status());
                                exec.approval.status = approval_receipt.status();
                                exec.swap.status = swap_receipt.status();
                                exec.swap.receipt = Some(swap_receipt);
                                exec.approval.receipt = Some(approval_receipt);
                            }
                            _ => {
                                tracing::error!("Failed to get receipt");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to send swap transaction: {:?}", e);
                        exec.swap.error = Some(e.to_string());
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to send approval transaction: {:?}", e);
                exec.approval.error = Some(e.to_string());
            }
        }

        results.push(exec);
    }

    Ok(results)
}

pub async fn default_pre_exec_hook(exec_name: &str, _config: &crate::types::config::MarketMakerConfig) {
    tracing::info!("[{}] pre-exec hook", exec_name);
}

pub async fn default_post_exec_hook(exec_name: &str, _config: &crate::types::config::MarketMakerConfig) {
    tracing::info!("[{}] post-exec hook", exec_name);
}

pub mod chain;
