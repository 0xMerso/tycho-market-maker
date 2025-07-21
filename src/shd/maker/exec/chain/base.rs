use async_trait::async_trait;

use crate::{
    maker::exec::{default_post_exec_hook, default_pre_exec_hook, default_simulate},
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::{ExecutedPayload, PreparedTransaction},
    },
};

use super::super::{default_broadcast, ExecStrategy};

/// Base L2 execution strategy - optimized for Base network
/// The flashblock concept was developed by the Flashbots team. Flashblocks is one of two extensions provided in the launch of Rollup-Boost. Rollup-Boost is a platform built for Optimism-based (layer 2) rollup chains that allows chain operators to upgrade the sequencer1 with additional features.
/// The previous sequencing ("vanilla") was accomplished by running a priority fee auction of pending transactions every 2s, and building the block using op-geth.
/// The new sequencing uses a priority fee auction every 200ms, and building the block using op-rbuilder.
///
/// The new approach has some subtleties:
/// - Each flashblock represents the transaction ordering for a **portion** of a coming block
/// - Each flashblock has a built-in gas limit based on its index in the sequence
/// - Once a flashblock is broadcast, its transaction ordering will be reflected in the final block
/// - The sequence of flashblocks is **fixed**, a flashblock cannot preempt another one
pub struct BaseExec;

impl BaseExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for BaseExec {
    fn name(&self) -> &'static str {
        "BaseExec"
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

    async fn simulate(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Result<Vec<PreparedTransaction>, String> {
        tracing::info!("🔵 [{}] Simulating {} transactions", self.name(), transactions.len());
        Ok(default_simulate(transactions, &config, env).await)
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<ExecutedPayload>, String> {
        tracing::info!("🔵 [BaseExec] Broadcasting {} transactions on Base L2 for instance {}", prepared.len(), mmc.id());
        default_broadcast(prepared, mmc, env).await
    }
}
