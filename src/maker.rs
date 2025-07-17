use std::collections::HashMap;
use std::sync::Arc;

use futures::FutureExt;
use shd::error::{MarketMakerError, Result};
use shd::types::config::MarketMakerConfig;
use shd::{
    maker::{exec::ExecStrategyFactory, feed::PriceFeedFactory},
    types::{builder::MarketMakerBuilder, config::EnvConfig, maker::IMarketMaker, moni::NewInstanceMessage, tycho::TychoStreamState},
    utils::constants::RESTART,
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tycho_simulation::models::Token;

/// =============================================================================
/// Main Market Maker Runtime Loop
/// =============================================================================
///
/// @description: Infinite loop that runs the market maker with automatic restart on failure
/// @param mk: Market maker instance implementing IMarketMaker trait
/// @param identifier: Unique identifier for this market maker instance
/// @param config: Market maker configuration
/// @param env: Environment configuration
/// @param tokens: List of available tokens from Tycho API
/// @return Result<()>: Success or error
///
/// @behavior:
/// - Publishes instance start event if configured
/// - Fetches initial market price
/// - Runs infinite loop with panic recovery
/// - Automatically restarts on failure with configurable delay
/// =============================================================================
async fn run<M: IMarketMaker>(mut mk: M, identifier: String, config: MarketMakerConfig, env: EnvConfig, tokens: Vec<Token>) -> Result<()> {
    let commit = shd::utils::misc::commit().unwrap_or_default();

    // Publish instance start event if configured
    if config.publish_events {
        shd::data::r#pub::instance(NewInstanceMessage {
            config: config.clone(),
            identifier: identifier.clone(),
            commit: commit.clone(),
        });
    }

    // Fetch initial market price for validation
    if let Ok(price) = mk.fetch_market_price().await {
        tracing::info!("First market price: {:?} ({})", price, config.price_feed_config.r#type);
    } else {
        tracing::error!("Failed to fetch the first market price");
    }

    // ! ToDo: Add a check to see if the price is valid (not 0) and if both prices (reference and Tycho) are close to each other (within x% ?) at launch

    // Initialize shared state cache
    let cache = Arc::new(RwLock::new(TychoStreamState {
        protosims: HashMap::new(),
        components: HashMap::new(),
        atks: tokens.clone(),
    }));

    // Main runtime loop with automatic restart
    loop {
        tracing::debug!("Starting market make inf. loop (id: {}) and launching stream for network {}", identifier, config.network_name.as_str());
        tracing::info!("♻️  MarketMaker program commit: {:?}", commit.clone());

        let state = Arc::clone(&cache);
        match std::panic::AssertUnwindSafe(mk.run(state.clone(), env.clone())).catch_unwind().await {
            Ok(_) => tracing::debug!("Maker main task ended. Restarting..."),
            Err(e) => tracing::error!("Market maker task panicked: {:?}. Restarting...", e),
        }

        // Calculate restart delay (shorter in testing mode)
        let delay = if env.testing { RESTART / 10 } else { RESTART };
        tracing::debug!("Waiting {} seconds before restarting stream for {}", delay, config.network_name.as_str());
        tokio::time::sleep(tokio::time::Duration::from_millis(delay * 1000)).await;
    }
}

/// =============================================================================
/// Market Maker Initialization
/// =============================================================================
///
/// @description: Initialize and configure the market maker application
/// @return Result<()>: Success or error
///
/// @steps:
/// 1. Initialize logging and tracing
/// 2. Load environment configuration and secrets
/// 3. Load market maker configuration from TOML file
/// 4. Fetch tokens from Tycho API
/// 5. Validate base and quote tokens exist
/// 6. Create dynamic price feed and execution strategy
/// 7. Build and start market maker instance
/// =============================================================================
async fn initialize() -> Result<()> {
    // Initialize logging with environment-based configuration
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_max_level(Level::TRACE).with_env_filter(filter).init();

    // Load secrets from environment-specific file
    let path = std::env::var("CONFIG_PATH").unwrap();
    let path = path.replace(".toml", "");
    let path = path.replace("config/", "");
    let secrets = format!("config/secrets/.env.{}", path);
    tracing::info!("Loading secrets from: {}", secrets);

    // Load environment variables and validate configuration
    dotenv::from_filename(secrets).ok();
    let env = EnvConfig::new();
    env.print();

    // Load market maker configuration from TOML file
    tracing::info!("MarketMaker Config Path: '{}'", env.path);
    let config = match shd::types::config::load_market_maker_config(env.path.as_str()) {
        Ok(config) => config,
        Err(e) => return Err(MarketMakerError::Config(format!("Failed to load config: {}", e))),
    };
    config.print();
    tracing::debug!("🤖 MarketMaker Config Identifier: '{}'", config.identifier());

    // Validate network connectivity and get latest block
    let latest = shd::utils::evm::latest(config.rpc_url.clone()).await;
    tracing::info!("Launching Tycho Market Maker | 🧪 Testing mode: {:?} | Latest block: {}", env.testing, latest);

    if config.publish_events {
        tracing::info!("🏓  PublishEvent mode enabled. Publishing ping event to make sure Redis and Monitor are running");
        if let Err(e) = shd::data::r#pub::ping() {
            tracing::error!("Failed to publish ping event: {}", e);
            std::process::exit(1);
        } else {
            tracing::info!("Ping event published successfully");
        }
    }

    // Fetch available tokens from Tycho API
    let tokens = shd::maker::tycho::tokens(config.clone(), Some(env.tycho_api_key.as_str()))
        .await
        .ok_or_else(|| MarketMakerError::Config("Failed to fetch tokens from Tycho API".into()))?;

    // Validate base and quote tokens exist in the token list
    let base = tokens
        .iter()
        .find(|t| t.address.to_string() == config.base_token_address.to_lowercase())
        .ok_or_else(|| MarketMakerError::TokenNotFound(format!("Base token not found: {}", config.base_token_address)))?;

    let quote = tokens
        .iter()
        .find(|t| t.address.to_string() == config.quote_token_address.to_lowercase())
        .ok_or_else(|| MarketMakerError::TokenNotFound(format!("Quote token not found: {}", config.quote_token_address)))?;

    tracing::info!("Base token: {} | Quote token: {}", base.symbol, quote.symbol);

    // Create dynamic components based on configuration
    let feed = PriceFeedFactory::create(config.price_feed_config.r#type.as_str());
    let execution = ExecStrategyFactory::create(config.network_name.as_str());

    // Build market maker instance with all components
    let mk = MarketMakerBuilder::create(config.clone(), feed, execution, base.clone(), quote.clone()).map_err(|e| MarketMakerError::Config(format!("Failed to build Market Maker: {}", e)))?;

    let identifier = mk.identifier.clone();
    let _ = run(mk, identifier, config, env, tokens).await;

    Ok(())
}

/// =============================================================================
/// Application Entry Point
/// =============================================================================
///
/// @description: Main function that initializes and runs the market maker
/// @return: None (exits with error code 1 on failure)
/// =============================================================================
#[tokio::main]
async fn main() {
    if let Err(e) = initialize().await {
        tracing::error!("Market maker failed to start: {}", e);
        std::process::exit(1);
    }
}
