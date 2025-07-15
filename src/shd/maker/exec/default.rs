use std::str::FromStr;

use alloy::{
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
};
use alloy_primitives::B256;
use async_trait::async_trait;

use crate::{
    maker::tycho::get_alloy_chain,
    types::{
        config::{EnvConfig, MarketMakerConfig, NetworkName},
        maker::{ExecutedPayload, PreparedTransaction},
    },
    utils::constants::HAS_EXECUTED,
};

use super::ExecStrategy;

/// Default execution strategy - logs and returns transactions as-is
pub struct DefaultExec;

#[async_trait]
impl ExecStrategy for DefaultExec {
    /// Execute the transactions
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("🔧 [DefaultExec] Executing {} transactions with default strategy", transactions.len());

        // Simulate transactions first
        let simulated = self.simulate(config.clone(), transactions, env.clone()).await;
        tracing::info!("🔧 [DefaultExec] Simulation completed, {} transactions passed", simulated.len());

        if !simulated.is_empty() {
            let _ = self.broadcast(simulated.clone(), config, env).await;
        }

        simulated
    }

    /// Broadcast the transactions
    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) {
        let alloy_chain = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
        let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone(); // ! Custom per network
        let pk = env.wallet_private_key.clone();
        let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
        let signer = alloy::network::EthereumWallet::from(wallet.clone());
        let provider = ProviderBuilder::new().with_chain(alloy_chain).wallet(signer.clone()).on_http(rpc.clone());
        let mut exec = ExecutedPayload::default();

        let _network = NetworkName::from_str(mmc.network_name.as_str()).unwrap();

        for (x, tx) in prepared.iter().enumerate() {
            tracing::debug!("Trade: #{} | Broadcasting on {} | Method: {}", x, mmc.network_name.as_str().to_string(), mmc.broadcast_url);

            if HAS_EXECUTED.load(std::sync::atomic::Ordering::Relaxed) {
                // ! This is a hack to prevent the program from executing transactions in testing phase
                tracing::info!("⏩ Skipping broadcast ! Already executed a transaction in the program lifetime");
                return;
            }

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

    fn name(&self) -> &'static str {
        "DefaultExec"
    }
}
