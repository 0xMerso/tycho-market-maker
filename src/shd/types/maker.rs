use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;
use tycho_execution::encoding::models::Solution;
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
    fn spot_prices(&self, psc: &Vec<ProtoSimComp>) -> Vec<f64>;
    async fn evaluate(&self, psc: &Vec<ProtoSimComp>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment>;
    async fn readjust(&self, context: MarketContext, inventory: Inventory, crs: Vec<CompReadjustment>, env: EnvConfig) -> Vec<ExecutionOrder>;

    async fn fetch_inventory(&self, env: EnvConfig) -> Result<Inventory, String>;
    async fn fetch_market_context(&self, components: Vec<ProtocolComponent>, protosims: &HashMap<std::string::String, Box<dyn ProtocolSim>>, tokens: Vec<Token>) -> Option<MarketContext>;
    async fn fetch_eth_usd(&self) -> Result<f64, String>;
    async fn fetch_market_price(&self) -> Result<f64, String>;

    async fn solution(&self, order: ExecutionOrder, env: EnvConfig) -> Solution;
    fn prepare(&self, orders: Vec<ExecutionOrder>, solutions: Vec<Solution>, env: EnvConfig) -> Result<bool, String>;
    async fn simulate(&self, orders: Vec<ExecutionOrder>, solutions: Vec<Solution>, env: EnvConfig) -> Result<bool, String>;

    async fn execute(&self, order: Vec<ExecutionOrder>, context: MarketContext, inventory: Inventory, env: EnvConfig);
    async fn broadcast(&self);

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
    // Tycho
    pub psc: ProtoSimComp,
    // Recomputated
    pub direction: TradeDirection,
    pub selling: Token,
    pub buying: Token,
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
    pub gas_price_wei: u128,
}

#[derive(Debug, Clone)]
pub struct ExecutionOrder {
    pub adjustment: CompReadjustment,
    pub calculation: SwapCalculation,
    // pub bribing: BribeCalculation,
}

#[derive(Clone, Debug)]
pub struct SwapCalculation {
    pub base_to_quote: bool,
    pub selling_amount: f64,
    pub buying_amount: f64,
    pub powered_selling_amount: f64,
    pub powered_buying_amount: f64,
    // Post-swap price evaluation
    pub amount_out_divided: f64,
    pub amount_out_divided_min: f64,
    pub powered_amount_out_divided_min: f64,
    pub average_sell_price: f64,
    pub average_sell_price_net_gas: f64,
    // Gas
    pub gas_cost_eth: f64,
    pub gas_cost_usd: f64,
    pub gas_cost_in_output_token: f64,
    // Valuation
    pub selling_worth_usd: f64,
    pub buying_worth_usd: f64,
    // Profitability
    pub profit_delta_bps: f64,
    pub profitable: bool,
}
