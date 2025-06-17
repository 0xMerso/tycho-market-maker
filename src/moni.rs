use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;
use shd::{
    core::pricefeed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeed, PriceFeedType},
    types::{
        config::EnvConfig,
        maker::{IMarketMaker, MarketMakerBuilder},
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
    // let config = shd::types::config::load_market_maker_config("config/mmc.mainnet.toml");
    let config = shd::types::config::load_market_maker_config(env.path.as_str());
    config.print();
    let latest = shd::utils::evm::latest(config.rpc.clone()).await;
    tracing::info!("--- Launching MM Monitoring --- | 🧪 Testing mode: {:?} | Latest block: {}", env.testing, latest);
    // ============================================== Initialisation ==============================================
    // shd::utils::uptime::hearbeats(config.clone(), env.clone()).await;
}
