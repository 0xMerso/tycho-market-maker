use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::super::{evm, ExecStrategy};

/// Base L2 execution strategy - optimized for Base network
/// The flashblock concept was developed by the Flashbots team. Flashblocks is one of two extensions provided in the launch of Rollup-Boost. Rollup-Boost is a platform built for Optimism-based (layer 2) rollup chains that allows chain operators to upgrade the sequencer1 with additional features.
/// The previous sequencing (“vanilla”) was accomplished by running a priority fee auction of pending transactions every 2s, and building the block using op-geth.
/// The new sequencing uses a priority fee auction every 200ms, and building the block using op-rbuilder.
///
/// The new approach has some subtleties:
/// - Each flashblock represents the transaction ordering for a **portion** of a coming block
/// - Each flashblock has a built-in gas limit based on its index in the sequence
/// - Once a flashblock is broadcast, its transaction ordering will be reflected in the final block
/// - The sequence of flashblocks is **fixed**, a flashblock cannot preempt another one
/// - New
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
