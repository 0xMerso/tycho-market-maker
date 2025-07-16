use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::super::{evm, ExecStrategy};

/// Base L2 execution strategy - optimized for Base network
pub struct BaseExec;

impl BaseExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for BaseExec {
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("🔵 [BaseExec] Executing {} transactions on Base L2", transactions.len());
        // Simulate transactions first
        let simulated = self.simulate(config.clone(), transactions.clone(), env.clone()).await;
        tracing::info!("🔵 [BaseExec] Simulation completed, {} transactions passed", simulated.len());
        if !simulated.is_empty() {
            let _ = self.broadcast(simulated.clone(), config, env).await;
        }
        simulated
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) {
        tracing::info!("🔵 [BaseExec] Broadcasting {} transactions on Base L2", prepared.len());
        evm::broadcast(prepared, mmc, env).await;
    }

    fn name(&self) -> &'static str {
        "BaseExec"
    }
}
