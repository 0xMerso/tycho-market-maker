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
    let Ok(response) = reqwest::get(COINGECKO_ETH_USD).await else {
        return None;
    };

    response.json::<CoinGeckoResponse>().await.ok().map(|data| data.ethereum.usd)
}

/// === Binance ===
pub struct BinancePriceFeed;

#[async_trait]
impl PriceFeed for BinancePriceFeed {
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let symbol = format!("{}{}", mmc.base_token.to_uppercase(), mmc.quote_token.to_uppercase());
        let endpoint = format!("{}/ticker/price?symbol={}", mmc.price_feed_config.source, symbol);
        binance(endpoint).await
    }
}

/// Fetch the price of a token from Binance
pub async fn binance(endpoint: String) -> Result<f64, String> {
    let Ok(res) = reqwest::get(endpoint).await else {
        return Err("Failed to fetch from Binance".to_string());
    };

    if !res.status().is_success() {
        let msg = format!("Error fetching price from Binance: {}", res.status());
        tracing::error!("{}", msg);
        return Err(msg);
    }

    let Ok(data) = res.json::<BinancePrice>().await else {
        return Err("Error deserializing Binance response".to_string());
    };

    tracing::debug!("Price data fetched from Binance: {:?}", data);

    data.price.parse::<f64>().map_err(|e| format!("Error parsing price: {:?}", e))
}

#[derive(Debug, Deserialize)]
struct BinancePrice {
    price: String,
}

/// === Chainlink ===
pub struct ChainlinkPriceFeed;

#[async_trait]
impl PriceFeed for ChainlinkPriceFeed {
    async fn get(&self, mmc: MarketMakerConfig) -> Result<f64, String> {
        let price = chainlink(mmc.rpc_url.clone(), mmc.price_feed_config.source.clone()).await?;

        if mmc.price_feed_config.reverse { Ok(1. / price) } else { Ok(price) }
    }
}

/// Fetch the price of an oracle
pub async fn chainlink(rpc: String, pfeed: String) -> Result<f64, String> {
    let provider = ProviderBuilder::new().on_http(rpc.parse().unwrap());
    let pfeed: Address = pfeed.parse().unwrap();
    let client = Arc::new(provider);
    let oracle = IChainLinkPF::new(pfeed, client.clone());

    let price = oracle.latestAnswer().call().await;
    let precision = oracle.decimals().call().await;

    let (Ok(price), Ok(precision)) = (price, precision) else {
        return Err(format!("Error fetching price from chainlink oracle: {:?}", pfeed));
    };

    let power = 10f64.powi(precision._0 as i32);
    let price = price._0.to_string().parse::<f64>().unwrap();
    Ok(price / power)
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

        let Some(tokens) = crate::helpers::global::specific(config.clone(), Some("sampletoken"), vec![base, quote]).await else {
            tracing::error!("No tokens found");
            return;
        };

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
}
