use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;
use serde_json;
use shd::{
    core::pricefeed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeed, PriceFeedType},
    types::{
        config::{MarketMakerConfig, MoniEnvConfig},
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
    dotenv::from_filename("config/.env.moni.ex").ok(); // Use .env.ex for testing purposes
    let env = MoniEnvConfig::new();
    env.print();
    // let commit = shd::misc::commit();
    let mut configs = vec![];
    let paths = env.paths.split(",").collect::<Vec<&str>>();
    for path in paths.iter() {
        let config = shd::types::config::load_market_maker_config(path);
        let latest = shd::utils::evm::latest(config.rpc_url.clone()).await;
        tracing::info!(" - Config: {} | Latest block: {}", path, latest);
        config.print();
        configs.push(config);
    }
    tracing::info!("--- Launching MM monitoring program with {} mk2 instances --- | 🧪 Testing mode: {:?}", configs.len(), env.testing);
    // ============================================== Initialisation ==============================================
    // shd::utils::uptime::hearbeats(config.clone(), env.clone()).await;
    // ============================================== Start ==============================================
    tracing::info!("🐘 Init and test connection to Neon, Prisma, SeaORM, to PgSQL");
    match shd::data::neon::connect().await {
        Ok(db) => {
            tracing::info!("🐘 Neon connected");
            tracing::info!("🐘 Inserting {} bots into DB", configs.len());
            match shd::data::neon::pull::bots(&db).await {
                Ok(bots) => {
                    tracing::info!("🐘 Found {} bots in DB", bots.len());
                    for bot in bots.iter() {
                        let config = bot.config.clone();
                        let config: MarketMakerConfig = serde_json::from_str(&config.as_str().unwrap()).unwrap();
                        tracing::info!("Got config: {}", config.shortname());
                    }
                }
                Err(err) => {
                    tracing::error!("Error: {}", err);
                }
            }
            // for config in configs.iter() {
            //     let _ = shd::data::neon::create::bot(&db, config.clone()).await;
            // }
            tracing::info!("🐘 Starting infinite listening of the Redis pub-sub channel: {}, for MM events", configs.len());
            shd::data::receiver::listen();
        }
        Err(err) => {
            tracing::error!("Error: {}", err);
        }
    }
    tracing::info!("Monitoring program finished");
}
