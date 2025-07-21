use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::{ExecutedPayload, PreparedTransaction},
};

use super::super::{default_broadcast, post_exec_hook, pre_exec_hook, ExecStrategy};

/// Unichain execution strategy - optimized for Unichain network
/// https://docs.unichain.org/docs/technical-information/advanced-txn
pub struct UnichainExec;

impl UnichainExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for UnichainExec {
    fn name(&self) -> &'static str {
        "UnichainExec"
    }

    async fn pre_exec_hook(&self, config: &MarketMakerConfig) {
        tracing::info!("🔗 [{}] Pre-exec hook", self.name());
        pre_exec_hook(self.name(), config).await;
    }

    async fn post_exec_hook(&self, config: &MarketMakerConfig) {
        tracing::info!("🔗 [{}] Post-exec hook", self.name());
        post_exec_hook(self.name(), config).await;
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<ExecutedPayload>, String> {
        tracing::info!("🔗 [{}] Broadcasting {} transactions on Unichain", self.name(), prepared.len());
        default_broadcast(prepared, mmc, env).await
    }
}
