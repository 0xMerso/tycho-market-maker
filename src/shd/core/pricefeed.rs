use std::sync::Arc;

use alloy::providers::ProviderBuilder;
use alloy_primitives::Address;
use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    types::{config::MarketMakerConfig, sol::IChainLinkPF},
    utils::r#static::COINGECKO_ETH_USD,
};

#[async_trait]
pub trait PriceFeed: Send + Sync {
    /// Fetch the current market price of token0/token1 from an external feed (e.g. Chainlink, Binance).
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String>;
}

pub enum PriceFeedType {
    Chainlink,
    Binance,
}

impl PriceFeedType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "chainlink" => PriceFeedType::Chainlink,
            "binance" => PriceFeedType::Binance,
            _ => panic!("Unknown price feed type: {}", s),
        }
    }
}

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

/// Retrieve eth usd price
pub async fn coingecko() -> Option<f64> {
    match reqwest::get(COINGECKO_ETH_USD).await {
        Ok(response) => match response.json::<CoinGeckoResponse>().await {
            Ok(data) => Some(data.ethereum.usd),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

/// === Binance ===
pub struct BinancePriceFeed;

#[async_trait]
impl PriceFeed for BinancePriceFeed {
    /// @endpoint: Binance API
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let symbol = format!("{}{}", mmc.base_token.to_uppercase(), mmc.quote_token.to_uppercase());
        let endpoint = format!("{}/ticker/price?symbol={}", mmc.price_feed_config.source, symbol);
        // tracing::debug!("Fetching price from Binance at {:?}", endpoint);
        binance(endpoint).await
    }
}

/// Fetch the price of a token from Binance
pub async fn binance(endpoint: String) -> Result<f64, String> {
    match reqwest::get(endpoint).await {
        Ok(res) => {
            if res.status().is_success() {
                let result = res.json::<BinancePrice>().await;
                if let Err(e) = result {
                    eprintln!("Deserialization error: {:?}", e);
                    return Err(format!("Error deserializing response: {:?}", e));
                }
                let data = result.unwrap();
                tracing::debug!("Price data fetched from Binance: {:?}", data);
                match data.price.parse::<f64>() {
                    Ok(price) => Ok(price),
                    Err(e) => {
                        eprintln!("Parse error: {:?}", e);
                        Err(format!("Error parsing price: {:?}", e))
                    }
                }
            } else {
                let msg = format!("Error #2 fetching price from Binance: {}", res.status());
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
        Err(e) => Err(format!("Error fetching price from Binance: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
struct BinancePrice {
    price: String,
}

/// === Chainlink ===
pub struct ChainlinkPriceFeed;

#[async_trait]
impl PriceFeed for ChainlinkPriceFeed {
    /// @endpoint: RPC
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        // tracing::debug!("Fetching price from Chainlink at {:?}", mmc.pfc.source);
        chainlink(mmc.rpc_url.clone(), mmc.gas_token_chainlink_price_feed.clone()).await
    }
}

/// Fetch the price of and oracle
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
            Ok(price._0.as_u64() as f64 / power)
        }
        _ => {
            let msg = format!("Error fetching price from chainlink oracle: {:?}", pfeed);
            // tracing::error!("{}", msg);
            Err(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::pricefeed::ChainlinkPriceFeed,
        types::{
            config::load_market_maker_config,
            maker::{IMarketMaker, MarketMakerBuilder},
        },
    };

    use super::BinancePriceFeed;

    #[tokio::test]
    async fn test_price_feed() {
        let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).try_init();
        tracing::info!("Testing price feed");
        let config = load_market_maker_config("config/mmc.toml");
        let base = config.base_token_address.clone();
        let quote = config.quote_token_address.clone();
        let tokens = crate::helpers::global::specific(config.clone(), Some("sampletoken"), vec![base, quote]).await;
        match tokens {
            Some(tokens) => {
                let base = tokens[0].clone();
                let quote = tokens[1].clone();
                if config.price_feed_config.r#type == "binance" {
                    let feed = BinancePriceFeed;
                    let mk2 = MarketMakerBuilder::new(config, Box::new(feed)).build(base, quote).expect("Failed to build Market Maker");
                    let price = mk2.fetch_market_price().await.expect("Failed to fetch market price");
                    tracing::info!("Market Price: {:.3}", price);
                    assert!(price > 1500. && price < 3000., "Unexpected price value");
                } else if config.price_feed_config.r#type == "chainlink" {
                    let config = load_market_maker_config("config/mmc.toml");
                    let feed = ChainlinkPriceFeed;
                    let mk2 = MarketMakerBuilder::new(config, Box::new(feed)).build(base, quote).expect("Failed to build Market Maker");
                    let price = mk2.fetch_market_price().await.expect("Failed to fetch market price");
                    tracing::info!("Market Price Chainlink: {:?}", price);
                    assert!(price > 1500. && price < 3000., "Unexpected price value");
                }
            }
            None => {
                tracing::error!("No tokens found");
            }
        }
    }
}
