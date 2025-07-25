use crate::utils::{self, constants::BASIS_POINT_DENO};
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};

// Define local error types since we're not using the global error module
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

use super::maker::PriceFeedConfig;

/// Environment configuration expected
#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub path: String,
    pub testing: bool,
    // APIs
    pub heartbeat: String,
    pub tycho_api_key: String,
    // Wallet
    pub wallet_private_key: String,
}

/// Environment configuration expected
#[derive(Debug, Clone)]
pub struct MoniEnvConfig {
    pub testing: bool,
    pub heartbeat: String,
    pub database_url: String,
    pub database_name: String,
}

/// Enum for network
#[derive(Debug, Clone, Deserialize)]
pub enum NetworkName {
    Ethereum,
    Base,
    Unichain,
}

impl NetworkName {
    /// Convert a Network enum to a string
    pub fn as_str(&self) -> &str {
        match self {
            NetworkName::Ethereum => "ethereum",
            NetworkName::Base => "base",
            NetworkName::Unichain => "unichain",
        }
    }
    /// Convert a string to a NetworkName enum
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ethereum" => Some(NetworkName::Ethereum),
            "base" => Some(NetworkName::Base),
            "unichain" => Some(NetworkName::Unichain),
            _ => None,
        }
    }
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvConfig {
    pub fn new() -> Self {
        EnvConfig {
            path: std::env::var("CONFIG_PATH").unwrap(),
            testing: std::env::var("TESTING").unwrap() == "true",
            heartbeat: std::env::var("HEARTBEAT").unwrap(),
            wallet_private_key: std::env::var("WALLET_PRIVATE_KEY").unwrap(),
            tycho_api_key: std::env::var("TYCHO_API_KEY").unwrap(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.tycho_api_key.is_empty() {
            return Err(ConfigError::Config("TYCHO_API_KEY cannot be empty".into()));
        }
        if self.wallet_private_key.is_empty() {
            return Err(ConfigError::Config("WALLET_PRIVATE_KEY cannot be empty".into()));
        }
        Ok(())
    }

    pub fn print(&self) {
        tracing::info!("Environment Configuration:");
        tracing::info!("  Config Path: {}", self.path);
        tracing::info!("  Testing Mode: {}", self.testing);
        tracing::info!("  Heartbeat URL: {}", self.heartbeat);
        tracing::info!("  Tycho API Key: {}...", &self.tycho_api_key[..8.min(self.tycho_api_key.len())]);
        tracing::info!("  Wallet Private Key: {}...", &self.wallet_private_key[..8.min(self.wallet_private_key.len())]);
    }
}

impl Default for MoniEnvConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl MoniEnvConfig {
    pub fn new() -> Self {
        MoniEnvConfig {
            // paths: utils::misc::get("CONFIGS_PATHS"),
            testing: utils::misc::get("TESTING") == "true",
            heartbeat: utils::misc::get("HEARTBEAT"),
            database_url: utils::misc::get("DATABASE_URL"),
            database_name: utils::misc::get("DATABASE_NAME"),
        }
    }

    pub fn print(&self) {
        tracing::debug!("MoniEnvConfig:");
        // tracing::debug!("  Paths:                 {}", self.paths);
        tracing::debug!("  Testing:               {}", self.testing);
        tracing::debug!("  Heartbeat:             {}", self.heartbeat);
        tracing::debug!("  Database URL:          {}", self.database_url);
        tracing::debug!("  Database Name:         {}", self.database_name);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketMakerConfig {
    pub wallet_public_key: String,
    pub base_token: String,
    pub base_token_address: String,
    pub quote_token: String,
    pub quote_token_address: String,
    pub pair_tag: String,
    pub network_name: String,
    pub chain_id: u64,
    pub gas_token_symbol: String,
    pub gas_token_chainlink_price_feed: String,
    pub rpc_url: String,
    pub explorer_url: String,
    pub target_spread_bps: f64,
    pub min_exec_spread_bps: f64,
    pub max_slippage_pct: f64,
    pub profitability_check: bool,
    pub max_inventory_ratio: f64,
    pub tx_gas_limit: u64,
    pub block_offset: u64,
    pub inclusion_block_delay: u64,
    pub tycho_api: String,
    pub poll_interval_ms: u64,
    pub permit2_address: String,
    pub tycho_router_address: String,
    pub publish_events: bool,
    pub skip_simulation: bool,
    pub infinite_approval: bool,
    pub price_feed_config: PriceFeedConfig,
    pub min_publish_timeframe_ms: u64,
}

impl MarketMakerConfig {
    /// mk-instance-<network_name>-<chain_id>-<base_token_symbol>-<quote_token_symbol>-<pub_address>
    pub fn id(&self) -> String {
        let f7 = self.wallet_public_key[..9].to_string(); // 0x + 7 chars
        let msg = format!("mmc-{}-{}-{}-{}", self.network_name, self.base_token, self.quote_token, f7);
        msg.to_lowercase()
    }

    pub fn hash(&self) -> String {
        let serialized = serde_json::to_string(self).unwrap();
        let hash = alloy_primitives::keccak256(serialized.as_bytes());
        hash.to_string()
    }

    pub fn print(&self) {
        // Ultra warnings for negative spreads
        if self.target_spread_bps < 0.0 {
            tracing::warn!(
                "🚨 Target spread is NEGATIVE: {} bps! This will cause unprofitable execution (and drain the inventory) ! 🚨",
                self.target_spread_bps
            );
        }
        if self.min_exec_spread_bps < 0.0 {
            tracing::warn!(
                "🚨 Min exec spread is NEGATIVE: {} bps! This will cause unprofitable execution (and drain the inventory) ! 🚨",
                self.min_exec_spread_bps
            );
        }

        tracing::debug!("Market Maker Config:");
        tracing::debug!("  Network:               {} with ID {}", self.network_name, self.chain_id);
        tracing::debug!("  Tag:                   {}", self.pair_tag);
        tracing::debug!("  Base Token:            {} ({})", self.base_token, self.base_token_address);
        tracing::debug!("  Quote Token:           {} ({})", self.quote_token, self.quote_token_address);
        tracing::debug!("  Wallet Public Key:     {}", self.wallet_public_key);
        tracing::debug!("  RPC:                   {}", self.rpc_url);
        tracing::debug!("  Explorer:              {}", self.explorer_url);
        tracing::debug!("  Gas token:             {}", self.gas_token_symbol);
        tracing::debug!("  Gas Oracle Feed:       {}", self.gas_token_chainlink_price_feed);
        tracing::debug!("  Spread (bps):          {}", self.target_spread_bps);
        tracing::debug!("  🔸 Min exec spread (bps): {}", self.min_exec_spread_bps);
        tracing::debug!("  🔸 Max Slippage (%):      {}", self.max_slippage_pct);
        tracing::debug!("  🔸 Profitability Check:   {}", self.profitability_check);
        tracing::debug!("  Max Inventory Ratio:   {}", self.max_inventory_ratio);
        tracing::debug!("  Gas Limit:             {}", self.tx_gas_limit);
        tracing::debug!("  Block Offset:          {}", self.block_offset);
        tracing::debug!("  Inclusion Block Delay: {}", self.inclusion_block_delay);
        tracing::debug!("  Tycho API:             {}", self.tycho_api);
        tracing::debug!("  Poll Interval (ms):    {}", self.poll_interval_ms);
        tracing::debug!("  Permit2:               {}", self.permit2_address);
        tracing::debug!("  Tycho Router:          {}", self.tycho_router_address);
        tracing::debug!("  Publish Events:        {}", self.publish_events);
        tracing::debug!("  Min Publish Timeframe (ms): {}", self.min_publish_timeframe_ms);
        tracing::debug!("  Skip Simulation:       {}", self.skip_simulation);
        tracing::debug!("  Skip Approval:      {}", self.infinite_approval);
        tracing::debug!("  Price Feed Config:     {:?}", self.price_feed_config);
    }

    // Print network-[base_token-quote_token]-price_feed_config.name
    pub fn shortname(&self) -> String {
        format!("{}-{}-{}-{}", self.network_name, self.base_token, self.quote_token, self.price_feed_config.r#type)
    }

    pub fn validate(&self) -> Result<()> {
        if self.target_spread_bps > BASIS_POINT_DENO as f64 {
            return Err(ConfigError::Config("target_spread_bps must be ≤ 10000 BPS (100%)".into()));
        }
        if self.max_slippage_pct > 1. {
            return Err(ConfigError::Config("max_slippage_pct must be ≤ 1.0 (100%)".into()));
        }
        if !(0.0..=1.0).contains(&self.max_inventory_ratio) {
            return Err(ConfigError::Config("max_inventory_ratio must be between 0.0 and 1.0".into()));
        }
        Ok(())
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.poll_interval_ms)
    }
}

/// Load and validate market maker configuration
pub fn load_market_maker_config(path: &str) -> Result<MarketMakerConfig> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) => {
            return Err(ConfigError::Config(format!("Failed to read config file: {e}")));
        }
    };

    let config: MarketMakerConfig = match toml::from_str(&contents) {
        Ok(config) => config,
        Err(e) => {
            return Err(ConfigError::Config(format!("Failed to parse TOML: {e}")));
        }
    };

    match config.validate() {
        Ok(()) => Ok(config),
        Err(e) => Err(e),
    }
}
