use serde::Deserialize;
use std::{fs, time::Duration};

use crate::utils;

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
    pub wallet_public_key: String,
    pub wallet_private_key: String,
}

/// Environment configuration expected
#[derive(Debug, Clone)]
pub struct MoniEnvConfig {
    pub paths: String,
    pub testing: bool,
    // APIs
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
            path: utils::misc::get("CONFIG_PATH"),
            testing: utils::misc::get("TESTING") == "true",
            heartbeat: utils::misc::get("HEARTBEAT"),
            wallet_public_key: utils::misc::get("WALLET_PUBLIC_KEY"),
            wallet_private_key: utils::misc::get("WALLET_PRIVATE_KEY"),
            tycho_api_key: utils::misc::get("TYCHO_API_KEY"),
        }
    }

    pub fn print(&self) {
        tracing::debug!("Env Config:");
        tracing::debug!("  Testing:               {}", self.testing);
        tracing::debug!("  Heartbeat:             {}", self.heartbeat);
        tracing::debug!("  Public key:            {}", self.wallet_public_key);
        // tracing::debug!("  Private Key:           🤐");
        tracing::debug!("  Tycho API Key:         {}...", &self.tycho_api_key[..5]);
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
            paths: utils::misc::get("CONFIGS_PATHS"),
            testing: utils::misc::get("TESTING") == "true",
            heartbeat: utils::misc::get("HEARTBEAT"),
            database_url: utils::misc::get("DATABASE_URL"),
            database_name: utils::misc::get("DATABASE_NAME"),
        }
    }

    pub fn print(&self) {
        tracing::debug!("MoniEnvConfig:");
        tracing::debug!("  Paths:                 {}", self.paths);
        tracing::debug!("  Testing:               {}", self.testing);
        tracing::debug!("  Heartbeat:             {}", self.heartbeat);
        tracing::debug!("  Database Name:         {}", self.database_name);
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MarketMakerConfig {
    // Exact match with config (e.g. mmc.toml)
    pub token0: String,
    pub addr0: String,
    pub token1: String,
    pub addr1: String,
    pub tag: String,
    pub network: String,
    pub chainid: u64,
    pub gas_token: String,
    pub gas_token_chainlink: String,
    pub rpc: String,
    pub explorer: String,
    pub spread: u32,
    pub min_exec_spread: f64,
    // pub max_consistent_spread: u32, // security
    // pub min_profit_spread_threshold: u32 // trigger
    pub slippage: f64,
    pub profitability: bool,
    pub max_trade_allocation: f64,
    pub broadcast: String,
    pub depths: Vec<f64>, // Quoted
    pub gas_limit: u64,
    pub target_block_offset: u64,
    pub tycho_endpoint: String,
    pub poll_interval_ms: u64,
    pub permit2: String,
    pub tycho_router: String,
    pub pfc: PriceFeedConfig,
}

impl MarketMakerConfig {
    pub fn print(&self) {
        tracing::debug!("Market Maker Config:");
        tracing::debug!("  Network:               {} with ID {}", self.network.as_str(), self.chainid);
        tracing::debug!("  Tag:                   {}", self.tag);
        tracing::debug!("  Token0:                {} ({})", self.token0, self.addr0);
        tracing::debug!("  Token1:                {} ({})", self.token1, self.addr1);
        tracing::debug!("  RPC:                   {}", self.rpc);
        tracing::debug!("  Explorer:              {}", self.explorer);
        tracing::debug!("  Gas token:             {}", self.gas_token);
        tracing::debug!("  Gas token chainlink:   {}", self.gas_token_chainlink);
        tracing::debug!("  Spread (bps):          {}", self.spread);
        tracing::debug!("  min_exec_spread (bps): {}", self.min_exec_spread);
        tracing::debug!("  Slippage:              {} ({} in bps)", self.slippage, self.slippage * 10000.0);
        tracing::debug!("  Profitability Check:   {}", self.profitability);
        tracing::debug!("  Max Trade Allocation:  {}", self.max_trade_allocation);
        tracing::debug!("  Broadcast:             {}", self.broadcast);
        tracing::debug!("  Depths:                {:?}", self.depths);
        tracing::debug!("  Gas Limit:             {}", self.gas_limit);
        tracing::debug!("  Target Block Offset:   {}", self.target_block_offset);
        tracing::debug!("  Tycho Endpoint:        {}", self.tycho_endpoint);
        tracing::debug!("  Poll Interval (secs):  {}", self.poll_interval_ms);
        tracing::debug!("  Permit2:               {}", self.permit2);
        tracing::debug!("  Tycho Router:          {}", self.tycho_router);
        tracing::debug!("  Price Feed Config:     {:?}", self.pfc);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.spread > 10_000 {
            return Err("spread must be ≤ 10000 BPS (100%)".into());
        }
        if self.slippage > 1. {
            return Err("slippage must be ≤ 1.0 (100%)".into());
        }
        if !(0.0..=1.0).contains(&self.max_trade_allocation) {
            return Err("max_trade_allocation must be between 0.0 and 1.0".into());
        }
        // if broadcast mode not available for the network, reject.
        // ! Add tons of compatibility checks
        Ok(())
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.poll_interval_ms)
    }
}

pub fn load_market_maker_config(path: &str) -> MarketMakerConfig {
    let contents = fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {e}")).unwrap();
    let config: MarketMakerConfig = toml::from_str(&contents).map_err(|e| format!("Failed to parse TOML: {e}")).unwrap();
    config.validate().expect("Invalid configuration");
    config
}
