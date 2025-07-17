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
    pub fn new(config: super::config::MarketMakerConfig, feed: Box<dyn PriceFeed>, execution: Box<dyn ExecStrategy>) -> Self {
        Self { config, feed, execution }
    }

    pub fn identifier(&self) -> String {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        // Merging of config.identifier() and timestamp
        let identifier = format!("{}-instance-{}", self.config.identifier(), timestamp);
        identifier.to_string()
    }

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

    /// Create a market maker with dynamic execution strategy and dynamic feed
    pub fn create(config: super::config::MarketMakerConfig, feed: Box<dyn PriceFeed>, execution: Box<dyn ExecStrategy>, base: Token, quote: Token) -> Result<MarketMaker, String> {
        tracing::info!("Building MarketMaker with feed: {} and execution: {}", feed.name(), execution.name());
        let builder = Self::new(config, feed, execution);
        builder.build(base, quote)
    }
}
