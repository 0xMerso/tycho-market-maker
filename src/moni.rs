use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;
use shd::{
    core::pricefeed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeed, PriceFeedType},
    types::{
        config::MoniEnvConfig,
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
    let env = MoniEnvConfig::new();
    env.print();
    // let commit = shd::misc::commit();
    let mut configs = vec![];
    let paths = env.paths.split(",").collect::<Vec<&str>>();
    for path in paths.iter() {
        let config = shd::types::config::load_market_maker_config(path);
        let latest = shd::utils::evm::latest(config.rpc.clone()).await;
        tracing::info!(" - Config: {} | Latest block: {}", path, latest);
        config.print();
        configs.push(config);
    }
    tracing::info!("--- Launching MM monitoring program with {} mk2 instances --- | 🧪 Testing mode: {:?}", configs.len(), env.testing);
    // ============================================== Initialisation ==============================================
    // shd::utils::uptime::hearbeats(config.clone(), env.clone()).await;
    // ============================================== Start ==============================================
    shd::data::receiver::listen().await;
    tracing::info!("Monitoring program finished");
}
