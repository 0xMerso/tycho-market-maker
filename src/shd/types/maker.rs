use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
};

use crate::core::feed::PriceFeed;

use super::{
    config::{EnvConfig, MarketMakerConfig},
    tycho::SharedTychoStreamState,
};

#[async_trait]
pub trait IMarketMaker: Send + Sync {
    fn prices(&self, components: &[ProtocolComponent], pts: &HashMap<String, Box<dyn ProtocolSim>>) -> Vec<f64>;
    async fn evaluate(&self, components: Vec<ProtocolComponent>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment>;
    async fn readjust(&self, inventory: Inventory, orders: Vec<CompReadjustment>, env: EnvConfig);
    async fn inventory(&self, env: EnvConfig) -> Result<Inventory, String>;
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
    // Base token from Tycho Client
    pub base: Token,
    // Quote token from Tycho Client
    pub quote: Token,
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

    pub fn build(self, base: Token, quote: Token) -> Result<MarketMaker, String> {
        Ok(MarketMaker {
            ready: false,
            config: self.config,
            feed: self.feed,
            initialised: false,
            base,
            quote,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceFeedConfig {
    pub r#type: String, // "binance" or "chainlink"
    pub source: String, // https if type is "binance", of 0xAddress if type is "chainlink"
}

#[derive(Debug, Clone)]
pub enum TradeDirection {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct CompReadjustment {
    pub direction: TradeDirection,
    pub selling: Token,
    pub buying: Token,
    pub component: ProtocolComponent,
    pub spot: f64,
    pub reference: f64,
    pub spread: f64,
    pub spread_bps: f64,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub base_balance: u128,
    pub quote_balance: u128,
    pub nonce: u64,
}
