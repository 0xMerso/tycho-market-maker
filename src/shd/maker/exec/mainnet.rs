use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::ExecStrategy;

/// Mainnet execution strategy - optimized for mainnet with flashbots
pub struct MainnetExec {
    pub use_flashbots: bool,
    pub target_block_offset: u64,
}

impl MainnetExec {
    pub fn new(use_flashbots: bool, target_block_offset: u64) -> Self {
        Self { use_flashbots, target_block_offset }
    }
}

#[async_trait]
impl ExecStrategy for MainnetExec {
    async fn execute(&self, _config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, _env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!(
            "🌐 [MainnetExec] Executing {} transactions on mainnet (flashbots: {}, target_block_offset: {})",
            transactions.len(),
            self.use_flashbots,
            self.target_block_offset
        );

        for (i, tx) in transactions.iter().enumerate() {
            if self.use_flashbots {
                tracing::debug!("  Transaction {}: Will be submitted via Flashbots bundle", i);
            } else {
                tracing::debug!("  Transaction {}: Will be submitted via standard mempool", i);
            }
        }
        transactions
    }

    fn name(&self) -> &'static str {
        "MainnetExec"
    }
}
