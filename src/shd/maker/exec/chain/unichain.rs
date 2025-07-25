use async_trait::async_trait;

use crate::{
    maker::exec::ExecStrategyName,
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::{BroadcastData, SimulatedData, Trade, TradeStatus},
        moni::NewTradeMessage,
    },
};

use super::super::ExecStrategy;

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
    fn name(&self) -> String {
        ExecStrategyName::UnichainStrategy.as_str().to_string()
    }
}
