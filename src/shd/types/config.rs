use serde::{Deserialize, Serialize};
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
        tracing::debug!("  Database URL:          {}", self.database_url);
        tracing::debug!("  Database Name:         {}", self.database_name);
    }
}

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct MarketMakerConfig {
//     // Exact match with config (e.g. mmc.toml)
//     pub token0: String,
//     pub addr0: String,
//     pub token1: String,
//     pub addr1: String,
//     pub tag: String,
//     pub network: String,
//     pub chainid: u64,
//     pub gas_token: String,
//     pub gas_token_chainlink: String,
//     pub rpc: String,
//     pub explorer: String,
//     pub spread: u32,
//     pub min_exec_spread: f64,
//     // pub max_consistent_spread: u32, // security
//     // pub min_profit_spread_threshold: u32 // trigger
//     pub slippage: f64,
//     pub profitability: bool,
//     pub max_trade_allocation: f64,
//     pub broadcast: String,
//     pub depths: Vec<f64>, // Quoted
//     pub gas_limit: u64,
//     pub target_block_offset: u64,
//     pub tycho_endpoint: String,
//     pub poll_interval_ms: u64,
//     pub permit2: String,
//     pub tycho_router: String,
//     pub pfc: PriceFeedConfig,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketMakerConfig {
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
    pub target_spread_bps: u32,
    pub min_exec_spread_bps: f64,
    pub max_slippage_pct: f64,
    pub profitability_check: bool,
    pub max_inventory_ratio: f64,
    pub broadcast_url: String,
    pub quote_depths: Vec<f64>,
    pub tx_gas_limit: u64,
    pub block_offset: u64,
    pub tycho_api: String,
    pub poll_interval_ms: u64,
    pub permit2_address: String,
    pub tycho_router_address: String,
    pub price_feed_config: PriceFeedConfig,
}

impl MarketMakerConfig {
    pub fn print(&self) {
        tracing::debug!("Market Maker Config:");
        tracing::debug!("  Network:               {} with ID {}", self.network_name, self.chain_id);
        tracing::debug!("  Tag:                   {}", self.pair_tag);
        tracing::debug!("  Base Token:            {} ({})", self.base_token, self.base_token_address);
        tracing::debug!("  Quote Token:           {} ({})", self.quote_token, self.quote_token_address);
        tracing::debug!("  RPC:                   {}", self.rpc_url);
        tracing::debug!("  Explorer:              {}", self.explorer_url);
        tracing::debug!("  Gas token:             {}", self.gas_token_symbol);
        tracing::debug!("  Gas Oracle Feed:       {}", self.gas_token_chainlink_price_feed);
        tracing::debug!("  Spread (bps):          {}", self.target_spread_bps);
        tracing::debug!("  min_exec_spread (bps): {}", self.min_exec_spread_bps);
        tracing::debug!("  Max Slippage (%):      {}", self.max_slippage_pct);
        tracing::debug!("  Profitability Check:   {}", self.profitability_check);
        tracing::debug!("  Max Inventory Ratio:   {}", self.max_inventory_ratio);
        tracing::debug!("  Broadcast:             {}", self.broadcast_url);
        tracing::debug!("  Depths:                {:?}", self.quote_depths);
        tracing::debug!("  Gas Limit:             {}", self.tx_gas_limit);
        tracing::debug!("  Block Offset:          {}", self.block_offset);
        tracing::debug!("  Tycho API:             {}", self.tycho_api);
        tracing::debug!("  Poll Interval (ms):    {}", self.poll_interval_ms);
        tracing::debug!("  Permit2:               {}", self.permit2_address);
        tracing::debug!("  Tycho Router:          {}", self.tycho_router_address);
        tracing::debug!("  Price Feed Config:     {:?}", self.price_feed_config);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.target_spread_bps > 10_000 {
            return Err("target_spread_bps must be ≤ 10000 BPS (100%)".into());
        }
        if self.max_slippage_pct > 1. {
            return Err("max_slippage_pct must be ≤ 1.0 (100%)".into());
        }
        if !(0.0..=1.0).contains(&self.max_inventory_ratio) {
            return Err("max_inventory_ratio must be between 0.0 and 1.0".into());
        }
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
