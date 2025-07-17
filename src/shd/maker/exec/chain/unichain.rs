use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::super::{evm, ExecStrategy};

/// Unichain execution strategy - optimized for Unichain network
pub struct UnichainExec;

impl UnichainExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for UnichainExec {
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("🔗 [UnichainExec] Executing {} transactions on Unichain", transactions.len());
        let simulated = self.simulate(config.clone(), transactions.clone(), env.clone()).await;
        tracing::info!("🔗 [UnichainExec] Simulation completed, {} transactions passed", simulated.len());
        if !simulated.is_empty() {
            let _ = self.broadcast(simulated.clone(), config, env).await;
        }
        simulated
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) {
        tracing::info!("🔗 [UnichainExec] Broadcasting {} transactions on Unichain", prepared.len());
        evm::broadcast(prepared, mmc, env).await;
    }

    fn name(&self) -> &'static str {
        "UnichainExec"
    }
}
