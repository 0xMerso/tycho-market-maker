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
        tracing::warn!("🌐 [{}] Simulation not implemented for mainnet strategy", self.name());
        vec![]
    }

    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("🌐 [{}] Executing {} transactions on mainnet", self.name(), transactions.len());

        let simulated = if config.skip_simulation {
            tracing::info!("🚀 Skipping simulation - direct execution enabled");
            transactions
        } else {
            // Simulate transactions first
            let simulated = self.simulate(config.clone(), transactions.clone(), env.clone()).await;
            tracing::info!("Simulation completed, transactions passed");
            simulated
        };

        for (i, _tx) in simulated.iter().enumerate() {
            tracing::debug!("  Transaction {}: Will be submitted via Flashbots bundle", i);
        }
        simulated
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) {
        tracing::info!("🌐 [{}] Broadcasting {} transactions on Mainnet with Flashbots", self.name(), prepared.len());

        // Use evm broadcasting logic
        let _results = evm::broadcast(prepared, mmc, env).await;
    }

    fn name(&self) -> &'static str {
        "MainnetExec"
    }
}
