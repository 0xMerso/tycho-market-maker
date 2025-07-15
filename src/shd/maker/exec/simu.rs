use std::collections::HashMap;
use std::str::FromStr;

use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::simulate::{SimBlock, SimulatePayload},
    signers::local::PrivateKeySigner,
};
use alloy_primitives::B256;

use crate::types::{config::EnvConfig, maker::PreparedTransaction};

/// Simulate transactions to validate they will succeed before execution
/// Pure EVM simulation, no bundle, etc.
pub async fn simulate_transactions(transactions: Vec<PreparedTransaction>, config: &crate::types::config::MarketMakerConfig, env: EnvConfig) -> Vec<PreparedTransaction> {
    let initial_len = transactions.len();
    tracing::debug!("Simulating {} transactions (keeping only the first for now)", transactions.len());
    let alloy_chain = crate::maker::tycho::get_alloy_chain(config.network_name.as_str().to_string()).expect("Failed to get alloy chain");
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
    tracing::debug!("Removed {} transactions (criterias: not owned by the wallet)", removed);
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
                        tracing::trace!(" 🔮 Simulated Block {}:", block.inner.header.number);
                        for (x, scr) in block.calls.iter().enumerate() {
                            let name = names.get(x).unwrap();
                            tracing::trace!("  SimCallResult for '{}': Gas: {} | Simulation status: {}", name, scr.gas_used, scr.status);
                            if !scr.status {
                                let reason = scr.error.clone().unwrap().message;
                                tracing::error!("Simulation failed on SimCallResult on '{}'. No broadcast. Reason: {}", name, reason);
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
