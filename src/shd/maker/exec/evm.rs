use std::collections::HashMap;
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
        maker::{ExecutedPayload, PreparedTransaction},
    },
    utils::constants::HAS_EXECUTED,
};

/// Simulate transactions to validate they will succeed before execution
/// Pure EVM simulation, no bundle, etc.
pub async fn simulate(transactions: Vec<PreparedTransaction>, config: &MarketMakerConfig, env: EnvConfig) -> Vec<PreparedTransaction> {
    let initial_len = transactions.len();
    tracing::debug!("Simulating {} transactions (keeping only the first for now)", transactions.len());
    let alloy_chain = get_alloy_chain(config.network_name.as_str().to_string()).expect("Failed to get alloy chain");
    let rpc = config.rpc_url.parse::<url::Url>().unwrap().clone(); // ! Custom per network
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    tracing::debug!("Wallet: {:?}", wallet.address().to_string().to_lowercase());
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let mut transactions = transactions.clone();
    transactions.retain(|t| {
        let sender = t.approval.from.unwrap_or_default().to_string().to_lowercase();
        tracing::debug!(" - Sender: {:?}", sender);
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
        // ! Tmp: using only the first transaction for now
        let first = transactions.first().expect("No transactions found");
        let transactions = vec![first.clone()];
        for (x, tx) in transactions.iter().enumerate() {
            let calls = vec![tx.approval.clone(), tx.swap.clone()];
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
                            tracing::trace!(" - SimCallResult for '{}': Gas: {} | Simulation status: {}", name, scr.gas_used, scr.status);
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
pub async fn broadcast(prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) {
    let alloy_chain = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
    let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());
    let mut exec = ExecutedPayload::default();
    let _network = NetworkName::from_str(mmc.network_name.as_str()).unwrap();

    for (x, tx) in prepared.iter().enumerate() {
        tracing::debug!("Trade: #{} | Broadcasting on {}", x, mmc.network_name.as_str().to_string());

        if env.testing {
            tracing::info!("🧪 Skipping broadcast ! Testing mode enabled");
            return;
        }

        // --- Sending, without waiting for receipt ---
        let time = std::time::SystemTime::now();
        match provider.send_transaction(tx.approval.clone()).await {
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
                        let approve_receipt = approve.get_receipt().await;
                        let swap_receipt = swap.get_receipt().await;
                        let total_time = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Approval get_receipt + Swap get_receipt took {} ms", total_time);
                        match (approve_receipt, swap_receipt) {
                            (Ok(approval_receipt), Ok(swap_receipt)) => {
                                tracing::debug!("Approval receipt: status: {:?}", approval_receipt.status());
                                exec.approval.status = approval_receipt.status();
                                exec.swap.status = swap_receipt.status();
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
    }
}
