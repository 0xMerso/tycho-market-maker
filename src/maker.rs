use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;
use shd::{
    maker::{
        exec::DefaultExec,
        feed::{BinancePriceFeed, ChainlinkPriceFeed, PriceFeed, PriceFeedType},
    },
    types::{
        config::EnvConfig,
        maker::{IMarketMaker, MarketMakerBuilder},
        moni::NewInstanceMessage,
        tycho::TychoStreamState,
    },
    utils::r#static::RESTART,
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tycho_simulation::models::Token;

async fn run_market_maker<M: IMarketMaker>(mut mk: M, identifier: String, config: shd::types::config::MarketMakerConfig, env: EnvConfig, commit: String, tokens: Vec<Token>) {
    use futures::FutureExt;
    use shd::types::moni::NewInstanceMessage;
    use shd::types::tycho::TychoStreamState;
    use shd::utils::r#static::RESTART;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    shd::data::r#pub::instance(NewInstanceMessage {
        config: config.clone(),
        identifier: identifier,
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
        tracing::info!("Pushing message to Redis, new instance deployed");
        tracing::debug!("Launching stream for network {}", config.network_name.as_str());

        let state = Arc::clone(&cache);
        match std::panic::AssertUnwindSafe(mk.run(state.clone(), env.clone())).catch_unwind().await {
            Ok(_) => tracing::debug!("Maker main task ended. Restarting..."),
            Err(e) => tracing::error!("Monitoring task panicked: {:?}. Restarting...", e),
        }

        let delay = if env.testing { RESTART / 10 } else { RESTART };
        tracing::debug!("Waiting {} seconds before restarting stream for {}", delay, config.network_name.as_str());
        tokio::time::sleep(tokio::time::Duration::from_millis(delay * 1000)).await;
    }
}

#[tokio::main]
async fn main() {
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_max_level(Level::TRACE).with_env_filter(filter).init();

    dotenv::from_filename("config/.env").ok();
    let env = EnvConfig::new();
    env.print();

    tracing::info!("MarketMaker Config Path: '{}'", env.path);
    let config = shd::types::config::load_market_maker_config(env.path.as_str());
    config.print();
    tracing::debug!("🤖 MarketMaker Config Identifier: '{}'", config.identifier());

    let latest = shd::utils::evm::latest(config.rpc_url.clone()).await;
    tracing::info!("Launching Tycho Market Maker | 🧪 Testing mode: {:?} | Latest block: {}", env.testing, latest);

    let commit = shd::utils::misc::commit().unwrap_or_default();
    tracing::info!("♻️  MarketMaker program commit: {:?}", commit);

    let base = config.base_token_address.clone().to_lowercase();
    let quote = config.quote_token_address.clone().to_lowercase();

    let Some(tokens) = shd::maker::tycho::tokens(config.clone(), Some(env.tycho_api_key.as_str())).await else {
        tracing::error!("Tokens not found with Tycho Client");
        return;
    };

    let base = tokens
        .iter()
        .find(|t| t.address.to_string() == base)
        .unwrap_or_else(|| panic!("Base token not found in the list of tokens: {}", base));
    let quote = tokens
        .iter()
        .find(|t| t.address.to_string() == quote)
        .unwrap_or_else(|| panic!("Quote token not found in the list of tokens: {}", quote));

    tracing::info!("Base token: {} | Quote token: {}", base.symbol, quote.symbol);

    let strategy = DefaultExec;
    let pft = config.price_feed_config.r#type.as_str();
    let commit = shd::utils::misc::commit().unwrap_or_default();

    match PriceFeedType::from_str(pft) {
        PriceFeedType::Binance => {
            let feed = BinancePriceFeed;
            let builder = MarketMakerBuilder::new(config.clone(), feed, strategy);
            let identifier = builder.identifier();
            let mk = builder.build(base.clone(), quote.clone()).expect("Failed to build Market Maker with Binance feed");
            run_market_maker(mk, identifier, config, env, commit, tokens.clone()).await;
        }
        PriceFeedType::Chainlink => {
            let feed = ChainlinkPriceFeed;
            let builder = MarketMakerBuilder::new(config.clone(), feed, strategy);
            let identifier = builder.identifier();
            let mk = builder.build(base.clone(), quote.clone()).expect("Failed to build Market Maker with Chainlink feed");
            run_market_maker(mk, identifier, config, env, commit, tokens.clone()).await;
        }
    }
}
