/// =============================================================================
/// Market Maker Binary Entry Point
/// =============================================================================
///
/// @description: Main binary executable for the Tycho Market Maker. This module contains
/// the application entry point, initialization logic, and the main runtime loop that
/// orchestrates the market making operations across different blockchain networks.
/// =============================================================================
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
/// @function: init_allowance
/// @description: Handle allowance for base and quote tokens
/// @param config: Market maker configuration containing token addresses and router
/// @param env: Environment configuration with wallet credentials
/// @behavior: If infinite_approval is true, approves u128::MAX for both base and quote tokens on permit2_address
/// =============================================================================
async fn init_allowance(config: MarketMakerConfig, env: EnvConfig) {
    tracing::info!("config.infinite_approval: {:?}", config.infinite_approval);

    // Skip allowance check if skip_approval is enabled
    if !config.infinite_approval {
        tracing::info!("infinite_approval is false, skipping allowance check, and approving at each trade");
        return;
    }

    tracing::info!(
        "Checking allowance for {} on Permit2 {} | For {} and {}",
        config.wallet_public_key.clone(),
        config.permit2_address.clone(),
        config.base_token.clone(),
        config.quote_token.clone()
    );

    // Allowance
    let base_allowance = shd::utils::evm::allowance(
        config.rpc_url.clone(),
        config.wallet_public_key.clone(),
        config.permit2_address.clone(),
        config.base_token_address.clone(),
    )
    .await;

    let quote_allowance = shd::utils::evm::allowance(
        config.rpc_url.clone(),
        config.wallet_public_key.clone(),
        config.permit2_address.clone(),
        config.quote_token_address.clone(),
    )
    .await;

    match (base_allowance, quote_allowance) {
        (Ok(base_allowance), Ok(quote_allowance)) => {
            tracing::info!("Allowance: {:?} | {:?}", base_allowance, quote_allowance);
            // Check if allowance is enough (half max u128)
            let target = u128::MAX / 2;
            let amount = u128::MAX;
            if base_allowance < target {
                tracing::warn!("Base allowance is not enough: {} < {}", base_allowance, target);
                let _ = shd::utils::evm::approve(config.clone(), env.clone(), config.permit2_address.clone(), config.base_token_address.clone(), amount).await;
            } else {
                tracing::info!("Base allowance is enough: {} >= {}", base_allowance, target);
            }
            if quote_allowance < target {
                tracing::warn!("Quote allowance is not enough: {} < {}", quote_allowance, target);
                let _ = shd::utils::evm::approve(config.clone(), env.clone(), config.permit2_address.clone(), config.quote_token_address.clone(), amount).await;
            } else {
                tracing::info!("Quote allowance is enough: {} >= {}", quote_allowance, target);
            }
        }
        _ => {
            tracing::error!("Failed to get allowance");
        }
    }
}

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
        let _ = shd::data::r#pub::instance(NewInstanceMessage {
            config: config.clone(),
            identifier: identifier.clone(),
            commit: commit.clone(),
        });
    }

    // ! ToDo: Add a check to see if the price is valid (not 0) and if both prices (reference and Tycho) are close to each other (within x% ?) at launch

    // Initialize shared state cache
    let cache = Arc::new(RwLock::new(TychoStreamState {
        protosims: HashMap::new(),
        components: HashMap::new(),
        atks: tokens.clone(),
    }));

    // Spawn heartbeat task
    shd::utils::uptime::heartbeats(env.testing, env.heartbeat.clone()).await;

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
    // let path = std::env::var("CONFIG_PATH").unwrap();
    let path = std::env::var("SECRET_PATH").unwrap();
    // let path = path.replace(".toml", "");
    // let path = path.replace("config/", "");
    let secrets = path;
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
    tracing::debug!("🤖 MarketMaker Config Identifier: '{}'", config.id());

    if config.publish_events {
        tracing::info!("📕  PublishEvent mode enabled. Publishing ping event to make sure Redis and Monitor are running");

        const MAX_RETRIES: u32 = 5;
        const RETRY_DELAY_SECS: u64 = 5;

        let mut retry_count = 0;
        loop {
            match shd::data::r#pub::ping() {
                Ok(_) => {
                    tracing::info!("Ping event published successfully");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        tracing::error!("Failed to publish ping event after {} attempts: {}", MAX_RETRIES, e);
                        std::process::exit(1);
                    }
                    tracing::warn!(
                        "Failed to publish ping event (attempt {}/{}): {}. Retrying in {} seconds...",
                        retry_count,
                        MAX_RETRIES,
                        e,
                        RETRY_DELAY_SECS
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                }
            }
        }
    }

    // Validate network connectivity and get latest block
    let latest = shd::utils::evm::latest(config.rpc_url.clone()).await;
    tracing::info!("Launching Tycho Market Maker | 🧪 Testing mode: {:?} | Latest block: {}", env.testing, latest);

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
    let _mk = MarketMakerBuilder::create(config.clone(), feed, execution, base.clone(), quote.clone()).map_err(|e| MarketMakerError::Config(format!("Failed to build Market Maker: {}", e)))?;

    // Initialize allowance for base and quote tokens, if infinite_approval is true, we approve u128::MAX for both base and quote tokens
    let _ = init_allowance(config.clone(), env.clone()).await;

    // Fetch initial market price for validation
    if let Ok(price) = _mk.fetch_market_price().await {
        tracing::info!("First market price: {:?} ({})", price, config.price_feed_config.r#type);
    } else {
        tracing::error!("Failed to fetch the first market price");
    }

    let identifier = _mk.identifier.clone();
    let _ = run(_mk, identifier, config, env, tokens).await;

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
