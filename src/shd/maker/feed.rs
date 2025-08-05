use alloy::providers::ProviderBuilder;
use alloy_primitives::Address;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

use crate::types::{config::MarketMakerConfig, sol::IChainLinkPF};

// === SHARED/TRAITS ===

/// Price feed trait for handling different price feed methods
#[async_trait]
pub trait PriceFeed: Send + Sync {
    /// Fetch the current market price of token0/token1 from an external feed (e.g. Chainlink, Binance).
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String>;

    /// Get the feed name for logging
    fn name(&self) -> &'static str;
}

/// Dynamic price feed factory
pub struct PriceFeedFactory;

impl PriceFeedFactory {
    /// =============================================================================
    /// @function: create
    /// @description: Factory method to create price feed instances based on type string
    /// @param feed: Price feed type string ("chainlink" or "binance")
    /// @behavior: Returns boxed trait object of appropriate price feed implementation
    /// =============================================================================
    pub fn create(feed: &str) -> Box<dyn PriceFeed> {
        let feed = PriceFeedType::from_str(feed);
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

pub enum PriceFeedType {
    Chainlink,
    Binance,
}

impl PriceFeedType {
    /// =============================================================================
    /// @function: from_str
    /// @description: Parses string to PriceFeedType enum
    /// @param s: String to parse ("chainlink" or "binance")
    /// @behavior: Returns corresponding enum variant, panics on unknown type
    /// =============================================================================
    pub fn from_str(s: &str) -> Self {
        match s {
            "chainlink" => PriceFeedType::Chainlink,
            "binance" => PriceFeedType::Binance,
            _ => panic!("Unknown price feed type: {}", s),
        }
    }
    /// =============================================================================
    /// @function: as_str
    /// @description: Converts PriceFeedType enum to string representation
    /// @behavior: Returns lowercase string representation of the feed type
    /// =============================================================================
    pub fn as_str(&self) -> &str {
        match self {
            PriceFeedType::Chainlink => "chainlink",
            PriceFeedType::Binance => "binance",
        }
    }
}

// === CHAINLINK ===

pub struct ChainlinkPriceFeed;

#[async_trait]
impl PriceFeed for ChainlinkPriceFeed {
    // Fetch the price from chainlink and reverse it if configured as such
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

/// =============================================================================
/// @function: chainlink
/// @description: Fetches price from a Chainlink oracle contract
/// @param rpc: RPC endpoint URL for blockchain connection
/// @param pfeed: Chainlink price feed contract address
/// @behavior: Calls latestAnswer() and decimals() on oracle, returns normalized price
/// =============================================================================
pub async fn chainlink(rpc: String, pfeed: String) -> Result<f64, String> {
    let provider = ProviderBuilder::new().on_http(rpc.parse().unwrap());
    let pfeed: Address = pfeed.clone().parse().unwrap();
    let client = Arc::new(provider);
    let oracle = IChainLinkPF::new(pfeed, client.clone());
    let price = oracle.latestAnswer().call().await;
    let precision = oracle.decimals().call().await;
    match (price, precision) {
        (Ok(price), Ok(precision)) => {
            let power = 10f64.powi(precision._0 as i32);
            // tracing::debug!("Price fetched: {}", price._0.as_u64() as f64 / power);
            // Ok(price._0.as_u64() as f64 / power)
            let price = price._0.to_string().parse::<u128>().unwrap() as f64 / power;
            Ok(price)
        }
        _ => {
            let msg = format!("Error fetching price from chainlink oracle: {:?}", pfeed);
            tracing::error!("{}", msg);
            Err(msg)
        }
    }
}

// ToDo === Pyth ===

pub struct PythPriceFeed;

// === BINANCE ===

pub struct BinancePriceFeed;

#[async_trait]
impl PriceFeed for BinancePriceFeed {
    // Fetch the price from Binance
    // ! No reverse option for Binance, only for Chainlink
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let symbol = format!("{}{}", mmc.base_token.to_uppercase(), mmc.quote_token.to_uppercase());
        let endpoint = format!("{}/ticker/price?symbol={}", mmc.price_feed_config.source, symbol);
        binance(endpoint).await
    }

    fn name(&self) -> &'static str {
        "BinancePriceFeed"
    }
}

/// =============================================================================
/// @function: binance
/// @description: Fetches token price from Binance API
/// @param endpoint: Full Binance API endpoint URL with symbol parameter
/// @behavior: Makes HTTP request to Binance and parses price from JSON response
/// =============================================================================
async fn binance(endpoint: String) -> Result<f64, String> {
    let response = reqwest::get(&endpoint).await.map_err(|e| format!("Failed to fetch from Binance: {}", e))?;
    let data: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse Binance response: {}", e))?;
    data["price"].as_str().unwrap_or("0").parse::<f64>().map_err(|e| format!("Failed to parse price: {}", e))
}

// === COINGECKO === Not a reference feed, just for gas price in case of no chainlink gas feed configured (or error)

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CoinGeckoResponse {
    pub ethereum: CryptoPrice,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CryptoPrice {
    pub usd: f64,
}

/// =============================================================================
/// @function: coingecko_eth_usd
/// @description: Fetches ETH/USD price from CoinGecko API as fallback
/// @behavior: Queries CoinGecko simple price endpoint and returns ETH price in USD
/// =============================================================================
pub async fn coingecko_eth_usd() -> Option<f64> {
    let endpoint = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
    let Ok(response) = reqwest::get(endpoint).await else {
        return None;
    };
    response.json::<CoinGeckoResponse>().await.ok().map(|data| data.ethereum.usd)
}
