use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;
use shd::{
    core::pricefeed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeed, PriceFeedType},
    types::{
        config::EnvConfig,
        maker::{IMarketMaker, MarketMakerBuilder},
        moni::TradeEvent,
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
    let latest = shd::utils::evm::latest(config.rpc_url.clone()).await;
    tracing::info!("--- Launching Tycho Market Maker --- | 🧪 Testing mode: {:?} | Latest block: {}", env.testing, latest);
    // ============================================== Initialisation ==============================================
    // shd::utils::uptime::hearbeats(config.clone(), env.clone()).await;
    let pft = config.price_feed_config.r#type.as_str();
    let feed: Box<dyn PriceFeed> = match PriceFeedType::from_str(pft) {
        PriceFeedType::Binance => Box::new(BinancePriceFeed),
        PriceFeedType::Chainlink => Box::new(ChainlinkPriceFeed),
        // @dev Add your custom price feed here
    };
    // Monitoring transactions via shared cache via hashmap, no Redis
    let base = config.base_token_address.clone().to_lowercase();
    let quote = config.quote_token_address.clone().to_lowercase();
    match shd::helpers::global::tokens(config.clone(), Some(env.tycho_api_key.as_str())).await {
        Some(tokens) => {
            let base = tokens
                .iter()
                .find(|t| t.address.to_string() == base)
                .unwrap_or_else(|| panic!("Base token not found in the list of tokens: {}", base));
            let quote = tokens
                .iter()
                .find(|t| t.address.to_string() == quote)
                .unwrap_or_else(|| panic!("Quote token not found in the list of tokens: {}", quote));

            tracing::info!("Base  token: {} | Quote token: {}", base.symbol, quote.symbol);
            let mut mk = MarketMakerBuilder::new(config.clone(), feed)
                .build(base.clone(), quote.clone())
                .expect("Failed to build Market Maker with the given config");
            shd::core::inventory::wallet(config.clone(), env.clone()).await;
            if let Ok(price) = mk.fetch_market_price().await {
                tracing::info!("Market Price: {:?} ({})", price, config.price_feed_config.r#type);
            }
            let cache = Arc::new(RwLock::new(TychoStreamState {
                protosims: HashMap::new(),
                components: HashMap::new(),
                atks: tokens.clone(),
            }));
            loop {
                tracing::debug!("Launching stream for network {}", config.network_name.as_str());
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
                tracing::debug!("Waiting {} seconds before restarting stream for {}", delay, config.network_name.as_str());
                tokio::time::sleep(tokio::time::Duration::from_millis(delay * 1000)).await;
            }
        }
        None => {
            tracing::error!("Tokens not found with Tycho Client");
            return;
        }
    }
}
