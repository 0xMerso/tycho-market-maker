use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
};

use crate::core::pricefeed::PriceFeed;

use super::{
    config::{EnvConfig, MarketMakerConfig},
    tycho::{ProtoSimComp, SharedTychoStreamState},
};

#[async_trait]
pub trait IMarketMaker: Send + Sync {
    fn get_prices(&self, components: &[ProtocolComponent], pts: &HashMap<String, Box<dyn ProtocolSim>>) -> Vec<f64>;
    async fn evaluate(&self, components: Vec<ProtocolComponent>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment>;
    async fn readjust(&self, inventory: Inventory, crs: Vec<CompReadjustment>, env: EnvConfig);

    async fn fetch_inventory(&self, env: EnvConfig) -> Result<Inventory, String>;
    async fn fetch_market_context(&self, ethpts: Vec<ProtoSimComp>, components: Vec<ProtocolComponent>, tokens: Vec<Token>) -> Option<MarketContext>;
    async fn fetch_eth_usd(&self) -> Result<f64, String>;
    async fn fetch_market_price(&self) -> Result<f64, String>;

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

#[derive(Debug, Clone)]
pub struct MarketContext {
    pub base_to_eth: f64,
    pub quote_to_eth: f64,
    pub eth_to_usd: f64,
}

#[derive(Debug, Clone)]
pub struct ExecutionOrder {
    pub cr: CompReadjustment,
    pub base_to_quote: bool,
    pub powered_selling_amount: f64,
    pub powered_buying_amount: f64,
    pub powered_buying_amount_min_recv: f64,
}
