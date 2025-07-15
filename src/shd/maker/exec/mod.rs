use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

/// Execution strategy trait for handling different execution methods
#[async_trait]
pub trait ExecStrategy: Send + Sync {
    /// Execute the prepared transactions
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction>;

    /// Get the strategy name for logging
    fn name(&self) -> &'static str;
}

pub mod default;
pub mod mainnet;
pub mod pga;

pub use default::DefaultExec;
pub use mainnet::MainnetExec;
pub use pga::GasBribeExec;
