//! Price Feed Module
//!
//! Price feed implementations for fetching external market prices.
//! Supports Chainlink oracles and Binance API for real-time price discovery.
use alloy::providers::ProviderBuilder;
use alloy_primitives::Address;
use async_trait::async_trait;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;

use crate::types::{config::MarketMakerConfig, sol::IChainLinkPF};

/// Interface for external price feed implementations.
#[async_trait]
pub trait PriceFeed: Send + Sync {
    /// Fetches the current market price from the external feed.
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String>;

    /// Returns the feed name for logging purposes.
    fn name(&self) -> &'static str;
}

/// Factory for creating price feed instances dynamically.
pub struct PriceFeedFactory;

impl PriceFeedFactory {
    /// Creates a price feed instance based on the type string ("chainlink" or "binance").
    pub fn create(feed: &str) -> Box<dyn PriceFeed> {
        let feed = PriceFeedType::from_str(feed).expect("Invalid price feed type");
        match feed {
            PriceFeedType::Binance => {
                tracing::info!("📊 Creating BinancePriceFeed");
                Box::new(BinancePriceFeed)
            }
            PriceFeedType::Chainlink => {
                tracing::info!("🔗 Creating ChainlinkPriceFeed");
                Box::new(ChainlinkPriceFeed)
            }
        }
    }
}

/// Available price feed types.
pub enum PriceFeedType {
    Chainlink,
    Binance,
}

impl FromStr for PriceFeedType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chainlink" => Ok(PriceFeedType::Chainlink),
            "binance" => Ok(PriceFeedType::Binance),
            _ => Err(format!("Unknown price feed type: {}", s)),
        }
    }
}

impl PriceFeedType {
    /// Converts to string representation.
    pub fn as_str(&self) -> &str {
        match self {
            PriceFeedType::Chainlink => "chainlink",
            PriceFeedType::Binance => "binance",
        }
    }
}

/// Chainlink oracle price feed implementation.
pub struct ChainlinkPriceFeed;

#[async_trait]
impl PriceFeed for ChainlinkPriceFeed {
    /// Fetches price from Chainlink oracle, optionally inverting if configured.
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let rev = mmc.price_feed_config.reverse;
        match chainlink(mmc.rpc_url.clone(), mmc.price_feed_config.source.clone()).await {
            Ok(price) => match rev {
                true => Ok(1. / price),
                false => Ok(price),
            },
            Err(e) => Err(e),
        }
    }

    fn name(&self) -> &'static str {
        "ChainlinkPriceFeed"
    }
}

/// Fetches price from a Chainlink oracle contract.
pub async fn chainlink(rpc: String, pfeed: String) -> Result<f64, String> {
    let provider = ProviderBuilder::new().connect_http(rpc.parse().unwrap());
    let pfeed: Address = pfeed.clone().parse().unwrap();
    let client = Arc::new(provider);
    let oracle = IChainLinkPF::new(pfeed, client.clone());
    let price = oracle.latestAnswer().call().await;
    let precision = oracle.decimals().call().await;
    match (price, precision) {
        (Ok(price), Ok(precision)) => {
            // Alloy 1.0: decimals() returns u8 directly, latestAnswer() returns I256 directly
            let power = 10f64.powi(precision as i32);
            let price = price.to_string().parse::<u128>().unwrap() as f64 / power;
            Ok(price)
        }
        _ => {
            let msg = format!("Error fetching price from chainlink oracle: {:?}", pfeed);
            tracing::error!("{}", msg);
            Err(msg)
        }
    }
}

/// Pyth network price feed implementation (placeholder).
pub struct PythPriceFeed;

/// Binance exchange price feed implementation.
pub struct BinancePriceFeed;

#[async_trait]
impl PriceFeed for BinancePriceFeed {
    /// Fetches spot price from Binance API.
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let symbol = format!("{}{}", mmc.base_token.to_uppercase(), mmc.quote_token.to_uppercase());
        let endpoint = format!("{}/ticker/price?symbol={}", mmc.price_feed_config.source, symbol);
        binance(endpoint).await
    }

    fn name(&self) -> &'static str {
        "BinancePriceFeed"
    }
}

/// Fetches token price from Binance API.
async fn binance(endpoint: String) -> Result<f64, String> {
    let response = reqwest::get(&endpoint).await.map_err(|e| format!("Failed to fetch from Binance: {}", e))?;
    let data: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse Binance response: {}", e))?;
    data["price"].as_str().unwrap_or("0").parse::<f64>().map_err(|e| format!("Failed to parse price: {}", e))
}

/// Response structure for CoinGecko API price data.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CoinGeckoResponse {
    pub ethereum: CryptoPrice,
}

/// Price structure containing USD value.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CryptoPrice {
    pub usd: f64,
}

/// Fetches ETH/USD price from CoinGecko API as fallback.
pub async fn coingecko_eth_usd() -> Option<f64> {
    let endpoint = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
    let Ok(response) = reqwest::get(endpoint).await else {
        return None;
    };
    response.json::<CoinGeckoResponse>().await.ok().map(|data| data.ethereum.usd)
}
