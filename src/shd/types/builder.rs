use tycho_simulation::models::Token;

use super::maker::MarketMaker;
use crate::maker::{exec::ExecStrategy, feed::PriceFeed};

/// Builder for creating MarketMaker instances
pub struct MarketMakerBuilder {
    config: super::config::MarketMakerConfig,
    feed: Box<dyn PriceFeed>,
    execution: Box<dyn ExecStrategy>,
}

impl MarketMakerBuilder {
    /// =============================================================================
    /// @function: new
    /// @description: Creates a new MarketMakerBuilder instance with configuration and strategies
    /// @param config: Market maker configuration containing network and token settings
    /// @param feed: Box containing the price feed strategy implementation
    /// @param execution: Box containing the execution strategy implementation
    /// @behavior: Initializes builder with provided components for later market maker construction
    /// =============================================================================
    pub fn new(config: super::config::MarketMakerConfig, feed: Box<dyn PriceFeed>, execution: Box<dyn ExecStrategy>) -> Self {
        Self { config, feed, execution }
    }

    /// =============================================================================
    /// @function: identifier
    /// @description: Generates a unique identifier for the market maker instance
    /// @behavior: Creates identifier from network, token pair, wallet address prefix, and timestamp
    /// =============================================================================
    pub fn identifier(&self) -> String {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        // Merging of config.identifier() and timestamp
        let f7 = self.config.wallet_public_key[..9].to_string(); // 0x + 7 chars
        let msg = format!("mmc-{}-{}-{}-{}", self.config.network_name, self.config.base_token, self.config.quote_token, f7);
        let identifier = format!("{}-instance-{}", msg.to_lowercase(), timestamp);
        identifier.to_string()
    }

    /// =============================================================================
    /// @function: build
    /// @description: Builds a MarketMaker instance from the configured builder
    /// @param base: Base token information from Tycho API
    /// @param quote: Quote token information from Tycho API
    /// @behavior: Consumes the builder and creates a configured MarketMaker instance
    /// =============================================================================
    pub fn build(self, base: Token, quote: Token) -> Result<MarketMaker, String> {
        let identifier = self.identifier();
        Ok(MarketMaker {
            ready: false,
            identifier,
            config: self.config,
            feed: self.feed,
            initialised: false,
            base,
            quote,
            single: false,
            execution: self.execution,
        })
    }

    /// =============================================================================
    /// @function: create
    /// @description: Static factory method to create a MarketMaker instance directly
    /// @param config: Market maker configuration
    /// @param feed: Price feed strategy implementation
    /// @param execution: Execution strategy implementation
    /// @param base: Base token information
    /// @param quote: Quote token information
    /// @behavior: Creates builder and immediately builds MarketMaker, logging strategy names
    /// =============================================================================
    pub fn create(config: super::config::MarketMakerConfig, feed: Box<dyn PriceFeed>, execution: Box<dyn ExecStrategy>, base: Token, quote: Token) -> Result<MarketMaker, String> {
        tracing::info!("Building MarketMaker with feed: {} and execution: {}", feed.name(), execution.name());
        let builder = Self::new(config, feed, execution);
        builder.build(base, quote)
    }
}
