use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::ExecStrategy;

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
