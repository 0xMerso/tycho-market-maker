//! MarketMaker Builder Module
use tycho_common::models::token::Token;

use super::maker::MarketMaker;
use crate::maker::{exec::ExecStrategy, feed::PriceFeed};

/// Builder for creating MarketMaker instances.
pub struct MarketMakerBuilder {
    config: super::config::MarketMakerConfig,
    feed: Box<dyn PriceFeed>,
    execution: Box<dyn ExecStrategy>,
}

impl MarketMakerBuilder {
    /// Creates a new MarketMakerBuilder with configuration and strategies.
    pub fn new(config: super::config::MarketMakerConfig, feed: Box<dyn PriceFeed>, execution: Box<dyn ExecStrategy>) -> Self {
        Self { config, feed, execution }
    }

    /// Generates a unique identifier for the market maker instance.
    ///
    /// Creates identifier from network, token pair, wallet address prefix, and timestamp.
    pub fn identifier(&self) -> String {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        // Merging of config.identifier() and timestamp
        let f7 = self.config.wallet_public_key[..9].to_string(); // 0x + 7 chars
        let msg = format!("mmc-{}-{}-{}-{}", self.config.network_name, self.config.base_token, self.config.quote_token, f7);
        let identifier = format!("{}-instance-{}", msg.to_lowercase(), timestamp);
        identifier.to_string()
    }

    /// Builds a MarketMaker instance from the configured builder.
    ///
    /// Consumes the builder and creates a configured MarketMaker instance.
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

    /// Static factory method to create a MarketMaker instance directly.
    ///
    /// Creates builder and immediately builds MarketMaker, logging strategy names.
    pub fn create(config: super::config::MarketMakerConfig, feed: Box<dyn PriceFeed>, execution: Box<dyn ExecStrategy>, base: Token, quote: Token) -> Result<MarketMaker, String> {
        tracing::info!("Building MarketMaker with feed: {} and execution: {}", feed.name(), execution.name());
        let builder = Self::new(config, feed, execution);
        builder.build(base, quote)
    }
}
