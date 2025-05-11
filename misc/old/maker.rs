use async_trait::async_trait;
use serde::Deserialize;

use crate::core::feed::PriceFeed;

use super::{config::MarketMakerConfig, tycho::SharedTychoStreamState};

#[async_trait]
pub trait IMarketMaker: Send + Sync {
    async fn market_price(&self) -> Result<f64, String>;
    async fn monitor(&self, state: SharedTychoStreamState);
}

/// ================== Market Maker ==================
pub struct MarketMaker<P: PriceFeed> {
    pub config: MarketMakerConfig,
    pub feed: P,
    // pub wallet: Wallet,
    // pub provider: RpcProvider,
    // pub tycho: TychoClient,
}

/// ================== Builder ==================
pub struct MarketMakerBuilder<P: PriceFeed> {
    config: MarketMakerConfig,
    feed: P,
}

impl<P: PriceFeed> MarketMakerBuilder<P> {
    pub fn new(config: MarketMakerConfig, feed: P) -> Self {
        Self { config, feed }
    }

    pub fn build(self) -> Result<MarketMaker<P>, String> {
        Ok(MarketMaker { config: self.config, feed: self.feed })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceFeedConfig {
    pub r#type: String,   // "binance" or "chainlink"
    pub endpoint: String, // if type is "binance"
    pub address: String,  // if type is "chainlink"
}
