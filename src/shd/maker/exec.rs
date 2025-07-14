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

/// Default execution strategy - logs and returns transactions as-is
pub struct DefaultExec;

#[async_trait]
impl ExecStrategy for DefaultExec {
    async fn execute(&self, _config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, _env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("🔧 [DefaultExec] Executing {} transactions with default strategy", transactions.len());
        for (i, tx) in transactions.iter().enumerate() {
            tracing::debug!(
                "  Transaction {}: Approval to {} | Swap to {}",
                i,
                tx.approval.to.as_ref().map(|t| format!("{:?}", t)).unwrap_or_else(|| "None".to_string()),
                tx.swap.to.as_ref().map(|t| format!("{:?}", t)).unwrap_or_else(|| "None".to_string())
            );
        }
        transactions
    }

    fn name(&self) -> &'static str {
        "DefaultExec"
    }
}

/// Gas bribe execution strategy - adds gas bribes for MEV protection
pub struct GasBribeExec {
    pub bribe_amount_wei: u128,
}

impl GasBribeExec {
    pub fn new(bribe_amount_wei: u128) -> Self {
        Self { bribe_amount_wei }
    }
}

#[async_trait]
impl ExecStrategy for GasBribeExec {
    async fn execute(&self, _config: MarketMakerConfig, mut transactions: Vec<PreparedTransaction>, _env: EnvConfig) -> Vec<PreparedTransaction> {
        tracing::info!("💰 [GasBribeExec] Executing {} transactions with gas bribe of {} wei", transactions.len(), self.bribe_amount_wei);

        for (i, tx) in transactions.iter_mut().enumerate() {
            // Add bribe to priority fee
            if let Some(current_priority) = tx.swap.max_priority_fee_per_gas {
                tx.swap.max_priority_fee_per_gas = Some(current_priority + self.bribe_amount_wei);
                tracing::debug!(
                    "  Transaction {}: Added {} wei bribe to priority fee ({} -> {})",
                    i,
                    self.bribe_amount_wei,
                    current_priority,
                    current_priority + self.bribe_amount_wei
                );
            }
        }
        transactions
    }

    fn name(&self) -> &'static str {
        "GasBribeExec"
    }
}

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

// https://docs.unichain.org/docs/technical-information/advanced-txn
