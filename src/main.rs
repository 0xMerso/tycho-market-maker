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
    env.log_config();
    // let commit = shd::misc::commit();
    let config = shd::types::config::load_market_maker_config("config/mmc.toml");
    config.log_config();
    tracing::info!("--- Launching Tycho Market Maker --- | 🧪 Testing mode: {:?}", env.testing);
    // ============================================== Initialisation ==============================================
    shd::data::ping().await;
    shd::data::set(keys::status(config.network.clone()).as_str(), StreamState::Launching as u128).await;
    // shd::utils::uptime::hearbeats(config.clone(), env.clone()).await;
    let pft = config.pfc.r#type.as_str();
    let feed: Box<dyn PriceFeed> = match PriceFeedType::from_str(pft) {
        PriceFeedType::Binance => Box::new(BinancePriceFeed),
        PriceFeedType::Chainlink => Box::new(ChainlinkPriceFeed),
        // @dev Add your custom price feed here
    };
    let mut mk = MarketMakerBuilder::new(config.clone(), feed).build().expect("Failed to build Market Maker with the given config");
    if let Ok(price) = mk.market_price().await {
        tracing::info!("Market Price: {:?}", price);
    }
    shd::core::inventory::wallet(config.clone(), env.clone()).await;
    // ? Fetch only the tokens that are in the config ?
    let specific = vec![config.addr0.clone(), config.addr1.clone()];
    let tokens = shd::core::helpers::specific(config.clone(), env.clone(), specific).await.unwrap_or_default();
    tracing::info!("Tokens ({}): {:?}", tokens.len(), tokens);
    if tokens.is_empty() {
        tracing::error!("Tokens in config were not found in Tycho database");
        return;
    }
    let cache = Arc::new(RwLock::new(TychoStreamState {
        protosims: HashMap::new(),
        components: HashMap::new(),
        tokens: tokens.clone(),
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
