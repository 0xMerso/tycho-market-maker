use async_trait::async_trait;

use crate::{
    maker::exec::simu::simulate_transactions,
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::PreparedTransaction,
    },
};

/// Execution strategy trait for handling different execution methods
#[async_trait]
pub trait ExecStrategy: Send + Sync {
    /// Execute the prepared transactions
    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction>;

    /// Simulate the transactions before execution
    async fn simulate(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Vec<PreparedTransaction> {
        // Default implementation uses the shared simulation logic
        simulate_transactions(transactions, &config, env).await
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
    pub fn create(broadcast_url: &str, block_offset: u64) -> Box<dyn ExecStrategy> {
        match broadcast_url {
            "flashbots" => {
                tracing::info!("🌐 Creating MainnetExec strategy with Flashbots");
                Box::new(mainnet::MainnetExec::new(true, block_offset))
            }
            "pga" | "gas-bribe" => {
                tracing::info!("🎯 Creating GasBribeExec strategy for PGA");
                let bribe_amount = 1_000_000_000; // 1 gwei in wei
                Box::new(pga::GasBribeExec::new(bribe_amount))
            }
            _ => {
                // Default: classic broadcasting
                tracing::info!("🔵 Creating DefaultExec strategy for classic broadcasting");
                Box::new(default::DefaultExec)
            }
        }
    }
}

pub mod default;
pub mod mainnet;
pub mod pga;
pub mod simu;
