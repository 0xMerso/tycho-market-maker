use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig, NetworkName},
    maker::PreparedTransaction,
};

/// Execution strategy trait for handling different execution methods
#[async_trait]
pub trait ExecStrategy: Send + Sync {
    /// Execute the prepared transactions
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction>;

    /// Simulate the transactions before execution
    async fn simulate(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        // Default implementation uses the shared simulation logic
        evm::simulate(transactions, &config, env).await
    }

    /// Broadcast the transactions
    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig);

    /// Get the strategy name for logging
    fn name(&self) -> &'static str;
}

/// Dynamic execution strategy factory
pub struct ExecStrategyFactory;

impl ExecStrategyFactory {
    /// Create the appropriate execution strategy based on broadcast URL configuration
    pub fn create(network: &str) -> Box<dyn ExecStrategy> {
        match NetworkName::from_str(network) {
            Some(NetworkName::Ethereum) => {
                tracing::info!("🌐 Creating MainnetExec strategy with Flashbots");
                Box::new(chain::mainnet::MainnetExec::new())
            }
            Some(NetworkName::Base) => {
                tracing::info!("🔵 Creating BaseExec strategy for Base L2");
                Box::new(chain::base::BaseExec::new())
            }
            Some(NetworkName::Unichain) => {
                tracing::info!("🔗 Creating UnichainExec strategy for Unichain");
                Box::new(chain::unichain::UnichainExec::new())
            }
            _ => {
                // Default: classic broadcasting (using BaseExec as fallback)
                tracing::warn!("⚠️ Unknown network '{}', using BaseExec strategy as fallback", network);
                panic!("Unknown network '{}', please check the network name in the config file", network);
            }
        }
    }
}

pub mod chain;
pub mod evm;
