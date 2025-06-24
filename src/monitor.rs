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
    utils::r#static::{CHANNEL_REDIS, RESTART},
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
    match shd::data::neon::connect(env.clone()).await {
        Ok(db) => {
            tracing::info!("🐘 Neon connected");

            // Pull configurations from DB
            match shd::data::neon::pull::configurations(&db).await {
                Ok(configurations) => {
                    tracing::info!("🐘 Found {} configurations in DB", configurations.len());
                }
                Err(err) => {
                    tracing::error!("Error: {}", err);
                }
            }

            // match shd::data::neon::pull::instances(&db).await {
            //     Ok(instances) => {
            //         tracing::info!("🐘 Found {} instances in DB", instances.len());
            //         for instance in instances.iter() {
            //             let config = instance.config.clone();
            //             let config: MarketMakerConfig = serde_json::from_str(&config.as_str().unwrap()).unwrap();
            //             tracing::info!(" - Instance: {} | Got config: {}", instance.id, config.shortname());
            //         }
            //         // Is running or not ?
            //         // ToDo
            //     }
            //     Err(err) => {
            //         tracing::error!("Error: {}", err);
            //     }
            // }

            tracing::info!("🐘 Starting infinite listening of the Redis pub-sub channel: {}, for MM events", CHANNEL_REDIS);
            shd::data::sub::listen(env.clone()).await;
        }
        Err(err) => {
            tracing::error!("Error: {}", err);
        }
    }
    tracing::info!("Monitoring program finished");
}
