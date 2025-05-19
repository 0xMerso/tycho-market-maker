use serde::Deserialize;
use std::{fs, time::Duration};

use crate::utils;

use super::maker::PriceFeedConfig;

/// Environment configuration expected
#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub testing: bool,
    // APIs
    pub heartbeat: String,
    pub tycho_api_key: String,
    // Wallet
    pub wallet_public_key: String,
    pub wallet_private_key: String,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvConfig {
    pub fn new() -> Self {
        EnvConfig {
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

#[derive(Debug, Deserialize, Clone)]
pub struct MarketMakerConfig {
    // Exact match with config (e.g. mmc.toml)
    pub token0: String,
    pub addr0: String,
    pub token1: String,
    pub addr1: String,
    pub network: String,
    pub gas_token: String,
    pub gas_token_chainlink: String,
    pub rpc: String,
    pub explorer: String,
    pub spread: u32,
    pub slippage: u32,
    pub profitability: bool,
    pub max_trade_allocation: f64,
    pub broadcast: String,
    pub sampdepths: Vec<f64>,
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
        tracing::debug!("  Network:               {}", self.network);
        tracing::debug!("  Token0:                {} ({})", self.token0, self.addr0);
        tracing::debug!("  Token1:                {} ({})", self.token1, self.addr1);
        tracing::debug!("  RPC:                   {}", self.rpc);
        tracing::debug!("  Explorer:              {}", self.explorer);
        tracing::debug!("  Gas token:             {}", self.gas_token);
        tracing::debug!("  Gas token chainlink:   {}", self.gas_token_chainlink);
        tracing::debug!("  Spread (bps):          {}", self.spread);
        tracing::debug!("  Slippage (bps):        {}", self.slippage);
        tracing::debug!("  Profitability Check:   {}", self.profitability);
        tracing::debug!("  Max Trade Allocation:  {}", self.max_trade_allocation);
        tracing::debug!("  Broadcast:             {}", self.broadcast);
        tracing::debug!("  Sampdepths:            {:?}", self.sampdepths);
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
        if self.slippage > 10_000 {
            return Err("slippage must be ≤ 10000 BPS (100%)".into());
        }
        if !(0.0..=1.0).contains(&self.max_trade_allocation) {
            return Err("max_trade_allocation must be between 0.0 and 1.0".into());
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
