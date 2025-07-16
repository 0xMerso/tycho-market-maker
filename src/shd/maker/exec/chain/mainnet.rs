use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::super::{evm, ExecStrategy};

/// Mainnet execution strategy - optimized for mainnet with flashbots
pub struct MainnetExec;

impl MainnetExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for MainnetExec {
    async fn simulate(&self, _config: MarketMakerConfig, _transactions: Vec<PreparedTransaction>, _env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::warn!("🌐 [MainnetExec] Simulation not implemented for mainnet strategy");
        vec![]
    }

    async fn execute(&self, _config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, _env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("🌐 [MainnetExec] Executing {} transactions on mainnet", transactions.len());

        for (i, _tx) in transactions.iter().enumerate() {
            tracing::debug!("  Transaction {}: Will be submitted via Flashbots bundle", i);
        }
        transactions
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) {
        tracing::info!("🌐 [MainnetExec] Broadcasting {} transactions on Mainnet with Flashbots", prepared.len());

        // Use evm broadcasting logic
        evm::broadcast(prepared, mmc, env).await;
    }

    fn name(&self) -> &'static str {
        "MainnetExec"
    }
}
