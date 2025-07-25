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
        maker::{BroadcastData, ReceiptData, SimulatedData, Trade},
    },
};

pub mod chain;

/// Execution strategy trait for handling different execution methods
#[async_trait]
pub trait ExecStrategy: Send + Sync {
    /// Get the strategy name for logging
    fn name(&self) -> &'static str;

    /// Pre-execution hook
    async fn pre_exec_hook(&self, config: &MarketMakerConfig);

    /// Post-execution hook
    async fn post_exec_hook(&self, config: &MarketMakerConfig, trades: Vec<Trade>, identifier: String);

    /// Execute the prepared transactions (orchestration)
    async fn execute(&self, config: MarketMakerConfig, trades: Vec<Trade>, env: EnvConfig, identifier: String) -> Result<Vec<Trade>, String>;

    /// Simulate transactions (validation)
    async fn simulate(&self, config: MarketMakerConfig, trades: Vec<Trade>, env: EnvConfig) -> Result<Vec<SimulatedData>, String>;

    /// Broadcast transactions (execution)
    async fn broadcast(&self, prepared: Vec<Trade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<BroadcastData>, String>;
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
// ! if not implement, it's default, implement that in the trait !

/// Simulate transactions to validate they will succeed before execution
/// Pure EVM simulation, no bundle, etc.
pub async fn default_simulate(transactions: Vec<Trade>, config: &MarketMakerConfig, env: EnvConfig) -> Vec<SimulatedData> {
    let initial = transactions.len();
    // tracing::debug!("default_simulate: {} transactions", transactions.len());
    let alloy_chain = get_alloy_chain(config.network_name.as_str().to_string()).expect("Failed to get alloy chain");
    let rpc = config.rpc_url.parse::<url::Url>().unwrap().clone(); // ! Custom per network
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    tracing::debug!("Wallet configured: {:?}", wallet.address().to_string().to_lowercase());
    let signer = alloy::network::EthereumWallet::from(wallet.clone());

    // --- Filtering transactions ---
    let mut transactions = transactions.clone();
    transactions.retain(|t| {
        let sender = t.approve.from.unwrap_or_default().to_string().to_lowercase();
        wallet.address().to_string().eq_ignore_ascii_case(sender.clone().as_str())
    });
    let removed = initial - transactions.len();
    if removed > 0 {
        tracing::debug!("Removed {} transactions (criterias: not owned by the wallet)", removed);
    }
    let mut simulations = HashMap::new();
    for i in 0..transactions.len() {
        simulations.insert(i, false);
    }
    let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());
    let mut output = vec![];

    for (_x, tx) in transactions.iter().enumerate() {
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
            return_full_transactions: true, // Faster without ?
        };
        let mut smd = SimulatedData::default();
        match provider.simulate(&payload).await {
            Ok(output) => {
                for block in output.iter() {
                    tracing::trace!("🔮 Simulated on block #{} ...", block.inner.header.number);
                    // ! No Loop, split approve/swap
                    if block.calls.len() == 2 {
                        for (x, scr) in block.calls.iter().enumerate() {
                            if x == 0 {
                                tracing::trace!(" - Approval simulation call skipped");
                                continue;
                            }
                            let name = names.get(x).unwrap();
                            let took = time.elapsed().as_millis();
                            tracing::trace!(" - SimCallResult for '{}': Gas: {} | Simulation status: {} | Took: {} ms", name, scr.gas_used, scr.status, took);
                            let now = std::time::Instant::now().elapsed().as_millis();
                            smd.simulated_at_ms = now;
                            smd.simulated_took_ms = took;
                            smd.estimated_gas = scr.gas_used as u128;
                            smd.status = scr.status;
                            smd.error = None;
                            if !scr.status {
                                let reason = scr.error.clone().unwrap().message;
                                tracing::error!(" - Simulation failed on SimCallResult on '{}'. No broadcast. Reason: {}", name, reason);
                                smd.error = Some(reason);
                            }
                        }
                    } else {
                        tracing::error!("Simulated invalid number of calls: {}", block.calls.len());
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to simulate: {:?}", e);
            }
        };
        output.push(smd);
    }
    output
}

/// Shared broadcasting function used by all strategies
pub async fn default_broadcast(prepared: Vec<Trade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<BroadcastData>, String> {
    tracing::debug!("default_broadcast: {} transactions", prepared.len());
    let alloy_chain = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
    let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());

    if env.testing {
        tracing::info!("🧪 Skipping broadcast ! Testing mode enabled");
        return Ok(Vec::new());
    }

    let mut output = Vec::new();
    for (x, tx) in prepared.iter().enumerate() {
        tracing::debug!("Tx: #{} | Broadcasting on {}", x, mmc.network_name.as_str().to_string());
        if tx.metadata.simulation.is_some() && tx.metadata.simulation.as_ref().unwrap().status == false {
            tracing::error!("Simulation failed for tx: #{}, no broadcast", x);
            continue;
        }
        let time = std::time::SystemTime::now();
        let mut bd = BroadcastData::default();
        match provider.send_transaction(tx.approve.clone()).await {
            Ok(approve) => {
                let took = time.elapsed().unwrap_or_default().as_millis() as u128;
                tracing::debug!("Explorer: {}tx/{} | Approval shoot took {} ms", mmc.explorer_url, approve.tx_hash(), took);
                let broadcasted = std::time::Instant::now().elapsed().as_millis();
                match provider.send_transaction(tx.swap.clone()).await {
                    Ok(swap) => {
                        let took = time.elapsed().unwrap_or_default().as_millis() as u128;
                        tracing::debug!("Explorer: {}tx/{} | Swap (+ approval) shoot took {} ms", mmc.explorer_url, swap.tx_hash(), took);
                        bd.broadcasted_at_ms = broadcasted;
                        bd.broadcasted_took_ms = took;
                        bd.hash = swap.tx_hash().to_string();
                        let approve_receipt = approve.get_receipt().await;
                        let swap_receipt = swap.get_receipt().await;
                        let total_time = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Approval get_receipt + Swap get_receipt took {} ms", total_time);
                        match (approve_receipt, swap_receipt) {
                            (Ok(approval_receipt), Ok(swap_receipt)) => {
                                tracing::debug!("Approval receipt: status: {:?}", approval_receipt.status());
                                // let confirmed_at_ms = std::time::Instant::now().elapsed().as_millis();
                                let swap_receipt = swap_receipt.clone();
                                let swap_receipt_data = ReceiptData {
                                    status: swap_receipt.status().clone(),
                                    gas_used: swap_receipt.gas_used,
                                    effective_gas_price: swap_receipt.effective_gas_price,
                                    error: None,
                                    transaction_hash: swap_receipt.transaction_hash.to_string(),
                                    transaction_index: swap_receipt.transaction_index.unwrap_or_default(),
                                    block_number: swap_receipt.block_number.clone().unwrap_or_default(),
                                };
                                bd.receipt = Some(swap_receipt_data);
                            }
                            (_, Err(e)) => {
                                tracing::error!("Failed to get receipt for swap transaction: {:?}", e.to_string());
                                bd.broadcast_error = Some(e.to_string());
                            }
                            _ => {
                                tracing::error!("Failed to get receipts, unhandled error");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to send swap transaction: {:?}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to send approval transaction: {:?}", e);
            }
        }
        output.push(bd);
    }
    Ok(output)
}

pub async fn default_pre_exec_hook(exec_name: &str, _config: &crate::types::config::MarketMakerConfig) {
    tracing::info!("[{}] default_pre_exec_hook", exec_name);
}

pub async fn default_post_exec_hook(exec_name: &str, _config: &crate::types::config::MarketMakerConfig) {
    tracing::info!("[{}] default_post_exec_hook", exec_name);
}
