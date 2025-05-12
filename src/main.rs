use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

use alloy::rpc::types::serde_helpers::quantity::vec;
use futures::FutureExt;
use shd::{
    core::feed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeed, PriceFeedType},
    data::keys,
    types::{
        config::EnvConfig,
        maker::{IMarketMaker, MarketMaker, MarketMakerBuilder},
        misc::StreamState,
        tycho::TychoStreamState,
    },
    utils::r#static::RESTART,
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // ============================================== Start ==============================================
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_max_level(Level::TRACE).with_env_filter(filter).init();
    dotenv::from_filename("config/.env").ok(); // Use .env.ex for testing purposes
    let env = EnvConfig::new();
    env.print();
    // let commit = shd::misc::commit();
    let config = shd::types::config::load_market_maker_config("config/mmc.toml");
    config.print();
    tracing::info!("--- Launching Tycho Market Maker --- | 🧪 Testing mode: {:?}", env.testing);
    // ============================================== Initialisation ==============================================
    // shd::utils::uptime::hearbeats(config.clone(), env.clone()).await;
    let pft = config.pfc.r#type.as_str();
    let feed: Box<dyn PriceFeed> = match PriceFeedType::from_str(pft) {
        PriceFeedType::Binance => Box::new(BinancePriceFeed),
        PriceFeedType::Chainlink => Box::new(ChainlinkPriceFeed),
        // @dev Add your custom price feed here
    };
    let base = config.addr0.clone();
    let quote = config.addr1.clone();
    match shd::core::helpers::specific(config.clone(), Some(env.tycho_api_key.as_str()), vec![base, quote]).await {
        Some(tokens) => {
            let base = tokens[0].clone();
            let quote = tokens[1].clone();
            tracing::info!("Base  token: {} | Quote token: {}", base.symbol, quote.symbol);
            let mut mk = MarketMakerBuilder::new(config.clone(), feed)
                .build(base, quote)
                .expect("Failed to build Market Maker with the given config");
            if let Ok(price) = mk.market_price().await {
                tracing::info!("Market Price: {:?}", price);
            }
            shd::core::inventory::wallet(config.clone(), env.clone()).await;
            let cache = Arc::new(RwLock::new(TychoStreamState {
                protosims: HashMap::new(),
                components: HashMap::new(),
            }));
            loop {
                tracing::debug!("Launching stream for network {}", config.network);
                let state = Arc::clone(&cache);
                match AssertUnwindSafe(mk.monitor(state.clone(), env.clone())).catch_unwind().await {
                    Ok(_) => {
                        tracing::debug!("Monitoring task ended. Restarting...");
                    }
                    Err(e) => {
                        tracing::error!("Monitoring task panicked: {:?}. Restarting...", e);
                    }
                }
                let delay = if env.testing { RESTART / 10 } else { RESTART };
                tracing::debug!("Waiting {} seconds before restarting stream for {}", delay, config.network);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay * 1000)).await;
            }
        }
        None => {
            tracing::error!("Tokens not found with Tycho Client");
            return;
        }
    }
}
