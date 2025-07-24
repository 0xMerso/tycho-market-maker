use async_trait::async_trait;

use crate::{
    maker::exec::default_simulate,
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::{ExecutedPayload, PreparedTransaction},
        moni::NewTradeMessage,
    },
};

use super::super::{default_broadcast, default_post_exec_hook, default_pre_exec_hook, ExecStrategy};

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
        "UnichainExec" // const
    }

    async fn pre_exec_hook(&self, config: &MarketMakerConfig) {
        tracing::info!("🔗 [{}] Pre-exec hook", self.name());
        default_pre_exec_hook(self.name(), config).await;
    }

    async fn post_exec_hook(&self, config: &MarketMakerConfig, transactions: Vec<ExecutedPayload>, identifier: String) {
        tracing::info!("🔗 [{}] Post-exec hook", self.name());

        tracing::info!("Saving trades for instance identifier: {}", identifier);
        for payload in transactions {
            match payload.clone().swap.receipt {
                Some(receipt) => {
                    let block = receipt.block_number.unwrap_or(0);
                    let _ = crate::data::r#pub::trade(NewTradeMessage {
                        identifier: identifier.clone(), // Use passed identifier for trade tracking
                        block,
                        payload: Some(payload.clone()),
                        trade_data: None,
                    });
                }
                None => {
                    tracing::error!("No receipt found for swap");
                }
            }
        }
        default_post_exec_hook(self.name(), config).await;
    }

    async fn execute(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig, identifier: String) -> Result<Vec<ExecutedPayload>, String> {
        self.pre_exec_hook(&config).await;
        tracing::info!("[{}] Executing {} transactions", self.name(), transactions.len());
        let simulated = if config.skip_simulation {
            tracing::info!("🚀 Skipping simulation - direct execution enabled");
            transactions
        } else {
            let simulated = self.simulate(config.clone(), transactions.clone(), env.clone()).await?;
            tracing::info!("Simulation completed, {} transactions passed", simulated.len());
            simulated
        };
        let results = if !simulated.is_empty() {
            self.broadcast(simulated.clone(), config.clone(), env).await?
        } else {
            vec![]
        };
        self.post_exec_hook(&config, results.clone(), identifier).await;
        Ok(results)
    }

    async fn simulate(&self, config: MarketMakerConfig, transactions: Vec<PreparedTransaction>, env: EnvConfig) -> Result<Vec<PreparedTransaction>, String> {
        tracing::info!("🔵 [{}] Simulating {} transactions", self.name(), transactions.len());
        Ok(default_simulate(transactions, &config, env).await) // ToDo
    }

    async fn broadcast(&self, prepared: Vec<PreparedTransaction>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<ExecutedPayload>, String> {
        tracing::info!("🔗 [{}] Broadcasting {} transactions on Unichain", self.name(), prepared.len());
        default_broadcast(prepared, mmc, env).await
    }
}
