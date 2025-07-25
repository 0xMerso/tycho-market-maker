use async_trait::async_trait;

use crate::{
    maker::exec::{default_post_exec_hook, default_pre_exec_hook, default_simulate},
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::{BroadcastData, SimulatedData, Trade, TradeStatus},
        moni::NewTradeMessage,
    },
};

use super::super::{default_broadcast, ExecStrategy};

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

    async fn post_exec_hook(&self, config: &MarketMakerConfig, trades: Vec<Trade>, identifier: String) {
        tracing::info!("🔗 [{}] Post-exec hook", self.name());

        tracing::info!("Saving trades for instance identifier: {}", identifier);

        for trade in trades {
            if trade.metadata.status != TradeStatus::BroadcastSucceeded {
                tracing::error!("Trade not broadcasted, skipping post-exec hook");
                continue;
            } else {
                let _ = crate::data::r#pub::trade(NewTradeMessage {
                    identifier: identifier.clone(), // Use passed identifier for trade tracking
                    data: trade.metadata.clone(),
                });
            }
        }

        default_post_exec_hook(self.name(), config).await;
    }

    async fn execute(&self, config: MarketMakerConfig, _trades: Vec<Trade>, env: EnvConfig, identifier: String) -> Result<Vec<Trade>, String> {
        self.pre_exec_hook(&config).await;
        tracing::info!("[{}] Executing {} trades", self.name(), _trades.len());
        let mut trades = _trades.clone();
        let mut trades_with_simu = if config.skip_simulation {
            tracing::info!("🚀 Skipping simulation - direct execution enabled");
            _trades
        } else {
            let smd = self.simulate(config.clone(), _trades.clone(), env.clone()).await?;
            for (x, smd) in smd.iter().enumerate() {
                trades[x].metadata.simulation = Some(smd.clone());
            }
            trades
        };

        // Set status to SimulationSucceeded for all trades
        for trade in trades_with_simu.iter_mut() {
            trade.metadata.status = TradeStatus::SimulationSucceeded;
        }

        let bd = self.broadcast(trades_with_simu.clone(), config.clone(), env).await?;
        for (x, bd) in bd.iter().enumerate() {
            trades_with_simu[x].metadata.broadcast = Some(bd.clone());
        }

        // Set status to SimulationSucceeded for all trades
        for trade in trades_with_simu.iter_mut() {
            trade.metadata.status = TradeStatus::BroadcastSucceeded;
        }

        self.post_exec_hook(&config, trades_with_simu.clone(), identifier).await;
        Ok(trades_with_simu)
    }

    async fn simulate(&self, config: MarketMakerConfig, trades: Vec<Trade>, env: EnvConfig) -> Result<Vec<SimulatedData>, String> {
        tracing::info!("🔵 [{}] Simulating {} trades", self.name(), trades.len());
        Ok(default_simulate(trades, &config, env).await) // ToDo
    }

    async fn broadcast(&self, prepared: Vec<Trade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<BroadcastData>, String> {
        tracing::info!("🔗 [{}] Broadcasting {} trades on Unichain", self.name(), prepared.len());
        default_broadcast(prepared, mmc, env).await
    }
}
