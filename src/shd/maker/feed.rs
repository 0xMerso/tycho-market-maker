/// =============================================================================
/// Price Feed Module
/// =============================================================================
///
/// @description: Price feed implementations for fetching external market prices.
/// Supports multiple price sources including Chainlink oracles and Binance API
/// for real-time price discovery in market making operations.
/// =============================================================================
use alloy::providers::ProviderBuilder;
use alloy_primitives::Address;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

use crate::types::{config::MarketMakerConfig, sol::IChainLinkPF};

// === SHARED/TRAITS ===

/// =============================================================================
/// @trait: PriceFeed
/// @description: Interface for external price feed implementations
/// @behavior: Provides standardized methods for fetching market prices
/// =============================================================================
#[async_trait]
pub trait PriceFeed: Send + Sync {
    /// =============================================================================
    /// @function: get
    /// @description: Fetch the current market price of token0/token1 from external feed
    /// @param mmc: Market maker configuration with feed settings
    /// @return Result<f64, String>: Price as float or error message
    /// =============================================================================
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String>;

    /// =============================================================================
    /// @function: name
    /// @description: Get the feed name for logging purposes
    /// @return &'static str: Name of the price feed implementation
    /// =============================================================================
    fn name(&self) -> &'static str;
}

/// =============================================================================
/// @struct: PriceFeedFactory
/// @description: Factory for creating price feed instances dynamically
/// @behavior: Creates appropriate price feed based on configuration
/// =============================================================================
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

/// =============================================================================
/// @enum: PriceFeedType
/// @description: Enumeration of available price feed types
/// @variants:
/// - Chainlink: On-chain oracle price feed
/// - Binance: Centralized exchange price feed
/// =============================================================================
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

/// =============================================================================
/// @struct: ChainlinkPriceFeed
/// @description: Chainlink oracle price feed implementation
/// @behavior: Fetches prices from on-chain Chainlink price feed contracts
/// =============================================================================
pub struct ChainlinkPriceFeed;

#[async_trait]
impl PriceFeed for ChainlinkPriceFeed {
    /// =============================================================================
    /// @function: get
    /// @description: Fetch price from Chainlink oracle and apply reversal if configured
    /// @param mmc: Market maker configuration with Chainlink settings
    /// @return Result<f64, String>: Oracle price or error
    /// @behavior: Queries Chainlink contract and optionally inverts price
    /// =============================================================================
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

    /// =============================================================================
    /// @function: name
    /// @description: Get the Chainlink price feed identifier
    /// @return &'static str: "ChainlinkPriceFeed"
    /// =============================================================================
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
            // Alloy 1.0: decimals() returns u8 directly, latestAnswer() returns I256 directly
            let power = 10f64.powi(precision as i32);
            let price = price.to_string().parse::<u128>().unwrap() as f64 / power;
            // tracing::debug!("Price fetched from {}: {}", pfeed, price);
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

/// =============================================================================
/// @struct: PythPriceFeed
/// @description: Pyth network price feed implementation (placeholder)
/// @behavior: Future implementation for Pyth oracle integration
/// =============================================================================
pub struct PythPriceFeed;

// === BINANCE ===

/// =============================================================================
/// @struct: BinancePriceFeed
/// @description: Binance exchange price feed implementation
/// @behavior: Fetches spot prices from Binance REST API
/// =============================================================================
pub struct BinancePriceFeed;

#[async_trait]
impl PriceFeed for BinancePriceFeed {
    /// =============================================================================
    /// @function: get
    /// @description: Fetch spot price from Binance API
    /// @param mmc: Market maker configuration with Binance settings
    /// @return Result<f64, String>: Current market price or error
    /// @behavior: Queries Binance ticker endpoint. Note: No reverse option for Binance
    /// =============================================================================
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let symbol = format!("{}{}", mmc.base_token.to_uppercase(), mmc.quote_token.to_uppercase());
        let endpoint = format!("{}/ticker/price?symbol={}", mmc.price_feed_config.source, symbol);
        binance(endpoint).await
    }

    /// =============================================================================
    /// @function: name
    /// @description: Get the Binance price feed identifier
    /// @return &'static str: "BinancePriceFeed"
    /// =============================================================================
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

// === COINGECKO ===

/// =============================================================================
/// @struct: CoinGeckoResponse
/// @description: Response structure for CoinGecko API price data
/// @behavior: Used as fallback for gas price when Chainlink feed unavailable
/// =============================================================================
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CoinGeckoResponse {
    pub ethereum: CryptoPrice,
}

/// =============================================================================
/// @struct: CryptoPrice
/// @description: Price structure containing USD value
/// @fields: usd - Price in USD
/// =============================================================================
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
