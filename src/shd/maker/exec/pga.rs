use async_trait::async_trait;

use crate::types::{
    config::{EnvConfig, MarketMakerConfig},
    maker::PreparedTransaction,
};

use super::ExecStrategy;

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
