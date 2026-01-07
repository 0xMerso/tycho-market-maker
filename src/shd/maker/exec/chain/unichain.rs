//! Unichain Execution Strategy
//!
//! Optimized for Unichain network with advanced transaction features.
//! See: <https://docs.unichain.org/docs/technical-information/advanced-txn>
use async_trait::async_trait;

use crate::maker::exec::ExecStrategyName;

use super::super::ExecStrategy;

/// Unichain execution strategy implementation.
pub struct UnichainExec;

impl Default for UnichainExec {
    fn default() -> Self {
        Self::new()
    }
}

impl UnichainExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecStrategy for UnichainExec {
    fn name(&self) -> String {
        ExecStrategyName::UnichainStrategy.as_str().to_string()
    }

    // TODO: Override broadcast() for Unichain advanced transaction features
    // async fn broadcast(&self, prepared: Vec<Trade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<BroadcastData>, String>
}
