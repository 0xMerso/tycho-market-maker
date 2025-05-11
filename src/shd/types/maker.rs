use async_trait::async_trait;
use serde::Deserialize;

use crate::core::feed::PriceFeed;

use super::{
    config::{EnvConfig, MarketMakerConfig},
    tycho::SharedTychoStreamState,
};

#[async_trait]
pub trait IMarketMaker: Send + Sync {
    async fn market_price(&self) -> Result<f64, String>;
    async fn monitor(&mut self, mtx: SharedTychoStreamState, env: EnvConfig);
}

/// ================== Market Maker ==================
pub struct MarketMaker {
    // Ready when the ProtocolStreamBuilder is initialised
    pub ready: bool,
    // Configuration for the market maker
    pub config: MarketMakerConfig,
    // Price feed to use for market price
    pub feed: Box<dyn PriceFeed>,
    // Indicates whether the ProtocolStreamBuilder has been initialised (true if first stream has been received and saved)
    pub initialised: bool,
    // pub wallet: Wallet,
    // pub provider: RpcProvider,
    // pub tycho: TychoClient,
}

/// ================== Builder ==================
pub struct MarketMakerBuilder {
    config: MarketMakerConfig,
    feed: Box<dyn PriceFeed>,
}

impl MarketMakerBuilder {
    pub fn new(config: MarketMakerConfig, feed: Box<dyn PriceFeed>) -> Self {
        Self { config, feed }
    }

    pub fn build(self) -> Result<MarketMaker, String> {
        Ok(MarketMaker {
            ready: false,
            config: self.config,
            feed: self.feed,
            initialised: false,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceFeedConfig {
    pub r#type: String, // "binance" or "chainlink"
    pub source: String, // https if type is "binance", of 0xAddress if type is "chainlink"
}
