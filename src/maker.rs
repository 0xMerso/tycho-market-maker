use std::{collections::HashMap, sync::Arc};

use futures::FutureExt;
use shd::error::{MarketMakerError, Result};
use shd::{
    maker::{
        exec::DefaultExec,
        feed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeedType},
    },
    types::{
        config::EnvConfig,
        maker::{IMarketMaker, MarketMakerBuilder},
        moni::NewInstanceMessage,
        tycho::TychoStreamState,
    },
    utils::constants::RESTART,
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tycho_simulation::models::Token;

async fn run_market_maker<M: IMarketMaker>(mut mk: M, identifier: String, config: shd::types::config::MarketMakerConfig, env: EnvConfig, commit: String, tokens: Vec<Token>) -> Result<()> {
    shd::data::r#pub::instance(NewInstanceMessage {
        config: config.clone(),
        identifier: identifier.clone(),
        commit: commit.clone(),
    });

    if let Ok(price) = mk.fetch_market_price().await {
        tracing::info!("Market Price: {:?} ({})", price, config.price_feed_config.r#type);
    }

    let cache = Arc::new(RwLock::new(TychoStreamState {
        protosims: HashMap::new(),
        components: HashMap::new(),
        atks: tokens.clone(),
    }));

    loop {
        tracing::info!("Starting market maker loop for {}", identifier);
        tracing::debug!("Launching stream for network {}", config.network_name.as_str());

        let state = Arc::clone(&cache);
        match std::panic::AssertUnwindSafe(mk.run(state.clone(), env.clone())).catch_unwind().await {
            Ok(_) => tracing::debug!("Maker main task ended. Restarting..."),
            Err(e) => tracing::error!("Market maker task panicked: {:?}. Restarting...", e),
        }

        let delay = if env.testing { RESTART / 10 } else { RESTART };
        tracing::debug!("Waiting {} seconds before restarting stream for {}", delay, config.network_name.as_str());
        tokio::time::sleep(tokio::time::Duration::from_millis(delay * 1000)).await;
    }
}

async fn initialize_market_maker() -> Result<()> {
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_max_level(Level::TRACE).with_env_filter(filter).init();

    dotenv::from_filename("config/.env").ok();
    let env = EnvConfig::new();
    env.print();

    tracing::info!("MarketMaker Config Path: '{}'", env.path);
    let config = match shd::types::config::load_market_maker_config(env.path.as_str()) {
        Ok(config) => config,
        Err(e) => return Err(MarketMakerError::Config(format!("Failed to load config: {}", e))),
    };
    config.print();
    tracing::debug!("🤖 MarketMaker Config Identifier: '{}'", config.identifier());

    let latest = shd::utils::evm::latest(config.rpc_url.clone()).await;
    tracing::info!("Launching Tycho Market Maker | 🧪 Testing mode: {:?} | Latest block: {}", env.testing, latest);

    let commit = shd::utils::misc::commit().unwrap_or_default();
    tracing::info!("♻️  MarketMaker program commit: {:?}", commit);

    let tokens = shd::maker::tycho::tokens(config.clone(), Some(env.tycho_api_key.as_str()))
        .await
        .ok_or_else(|| MarketMakerError::Config("Failed to fetch tokens from Tycho API".into()))?;

    let base = tokens
        .iter()
        .find(|t| t.address.to_string() == config.base_token_address.to_lowercase())
        .ok_or_else(|| MarketMakerError::TokenNotFound(format!("Base token not found: {}", config.base_token_address)))?;

    let quote = tokens
        .iter()
        .find(|t| t.address.to_string() == config.quote_token_address.to_lowercase())
        .ok_or_else(|| MarketMakerError::TokenNotFound(format!("Quote token not found: {}", config.quote_token_address)))?;

    tracing::info!("Base token: {} | Quote token: {}", base.symbol, quote.symbol);

    let strategy = DefaultExec;
    let pft = config.price_feed_config.r#type.as_str();

    match PriceFeedType::from_str(pft) {
        PriceFeedType::Binance => {
            let feed = BinancePriceFeed;
            let builder = MarketMakerBuilder::new(config.clone(), feed, strategy);
            let identifier = builder.identifier();
            let mk = builder
                .build(base.clone(), quote.clone())
                .map_err(|e| MarketMakerError::Config(format!("Failed to build Market Maker with Binance feed: {}", e)))?;
            run_market_maker(mk, identifier, config, env, commit, tokens).await?;
        }
        PriceFeedType::Chainlink => {
            let feed = ChainlinkPriceFeed;
            let builder = MarketMakerBuilder::new(config.clone(), feed, strategy);
            let identifier = builder.identifier();
            let mk = builder
                .build(base.clone(), quote.clone())
                .map_err(|e| MarketMakerError::Config(format!("Failed to build Market Maker with Chainlink feed: {}", e)))?;
            run_market_maker(mk, identifier, config, env, commit, tokens).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = initialize_market_maker().await {
        tracing::error!("Market maker failed to start: {}", e);
        std::process::exit(1);
    }
}
