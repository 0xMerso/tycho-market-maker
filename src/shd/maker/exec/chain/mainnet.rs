use std::str::FromStr;

use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::mev::{Inclusion, SendBundleRequest},
    signers::local::PrivateKeySigner,
};
use alloy_primitives::B256;
use async_trait::async_trait;

use crate::{
    maker::{
        exec::{default_post_exec_hook, default_pre_exec_hook},
        tycho::get_alloy_chain,
    },
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::{ExecutedPayload, PreparedTransaction},
    },
};

use alloy_mev::{BundleSigner, EthMevProviderExt, MevShareProviderExt};

use super::super::ExecStrategy;

/// Mainnet execution strategy - optimized for mainnet with flashbots
pub struct MainnetExec;

impl MainnetExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for MainnetExec {
    fn name(&self) -> &'static str {
        "MainnetExec"
    }

    async fn pre_exec_hook(&self, config: &MarketMakerConfig) {
        tracing::info!("🔗 [{}] Pre-exec hook", self.name());
        default_pre_exec_hook(self.name(), config).await;
    }

    async fn post_exec_hook(&self, config: &MarketMakerConfig, _transactions: Vec<ExecutedPayload>, _identifier: String) {
        tracing::info!("🔗 [{}] Post-exec hook", self.name());
        default_post_exec_hook(self.name(), config).await;
    }

    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig, identifier: String) -> Result<Vec<ExecutedPayload>, String> {
        self.pre_exec_hook(&config).await;
        tracing::info!("[{}] Executing {} transactions", self.name(), transactions.len());
        let simulated = if config.skip_simulation {
            tracing::info!("🚀 Skipping simulation - direct execution enabled");
            transactions
        } else {
            let simulated = self.simulate(config.clone(), transactions.clone(), env.clone()).await?;
            tracing::info!("Simulation completed, {} transactions passed", simulated.len());
            simulated
        };
        let transactions = if !simulated.is_empty() {
            self.broadcast(simulated.clone(), config.clone(), env).await?
        } else {
            vec![]
        };
        self.post_exec_hook(&config, transactions.clone(), identifier).await;
        Ok(transactions)
    }

    async fn simulate(&self, _config: MarketMakerConfig, _transactions: Vec<PreparedTransaction>, _env: EnvConfig) -> Result<Vec<PreparedTransaction>, String> {
        tracing::warn!("[{}] Simulation not implemented for mainnet strategy", self.name());
        Ok(vec![])
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<ExecutedPayload>, String> {
        tracing::info!("🌐 [{}] Broadcasting {} transactions on Mainnet with Flashbots for instance {}", self.name(), prepared.len(), mmc.id());

        let ac = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
        let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
        let pk = env.wallet_private_key.clone();
        let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
        let signer = alloy::network::EthereumWallet::from(wallet.clone());
        let provider = ProviderBuilder::new().with_chain(ac).wallet(signer.clone()).on_http(rpc.clone());

        // Added to alloy-mev because it's not supported yet
        let buildernet = "https://direct-us.buildernet.org:443".to_string();
        let bsigner = PrivateKeySigner::random(); // For now bundle signer is random
        let _brpc = provider
            .endpoints_builder()
            .authenticated_endpoint(buildernet.parse::<url::Url>().unwrap(), BundleSigner::flashbots(bsigner.clone()))
            .titan(BundleSigner::flashbots(bsigner.clone()))
            .beaverbuild()
            .flashbots(BundleSigner::flashbots(bsigner.clone()))
            .rsync()
            .build();

        let mut results = Vec::new();

        if env.testing {
            tracing::info!("🧪 Skipping broadcast ! Testing mode enabled");
            return Ok(results);
        }

        if prepared.len() == 1 {
            let tx = prepared.first().expect("No transaction found");
            let bnum = provider.get_block_number().await.expect("Failed to get block number");
            let target_block = bnum + mmc.inclusion_block_delay;
            tracing::info!(
                "🌐 [{}] Current block: {}, target inclusion block: {} (delay: {})",
                self.name(),
                bnum,
                target_block,
                mmc.inclusion_block_delay
            );

            match provider.build_bundle_item(tx.approval.clone(), false).await {
                Ok(approval) => match provider.build_bundle_item(tx.swap.clone(), false).await {
                    Ok(swap) => {
                        let bundle = SendBundleRequest {
                            bundle_body: vec![approval, swap],
                            inclusion: Inclusion::at_block(target_block),
                            ..Default::default()
                        };
                        match provider.send_mev_bundle(bundle, bsigner.clone()).await {
                            Ok(_) => {
                                tracing::info!("🌐 [{}] Bundle sent successfully", self.name());
                                // TODO: Add proper ExecutedPayload creation for flashbots
                                results.push(ExecutedPayload::default());
                            }
                            Err(e) => {
                                tracing::error!("🌐 [{}] Failed to send bundle: {:?}", self.name(), e);
                                return Err(format!("Failed to send bundle: {:?}", e));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("🌐 [{}] Failed to build swap bundle item: {:?}", self.name(), e);
                        return Err(format!("Failed to build swap bundle item: {:?}", e));
                    }
                },
                Err(e) => {
                    tracing::error!("🌐 [{}] Failed to build approval bundle item: {:?}", self.name(), e);
                    return Err(format!("Failed to build approval bundle item: {:?}", e));
                }
            }
        } else {
            return Err("MainnetExec only supports single transaction bundles".to_string());
        }

        Ok(results)
    }
}
