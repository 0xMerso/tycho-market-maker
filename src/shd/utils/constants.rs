//! Application constants and configuration values

/// Redis channel for pub/sub communication
pub const CHANNEL_REDIS: &str = "tycho_market_maker";

/// Restart delay in seconds
pub const RESTART: u64 = 5;

/// Basis point denominator (10000 = 100%)
pub const BASIS_POINT_DENO: f64 = 10_000.0;

/// Price move denominator
pub const PRICE_MOVE_DENO: f64 = 100.0;

/// Price move threshold
pub const PRICE_MOVE_THRESHOLD: f64 = 0.5;

/// Add TVL threshold
pub const ADD_TVL_THRESHOLD: f64 = 100_000.0;

/// Share pool balance swap basis points
pub const SHARE_POOL_BAL_SWAP_BPS: f64 = 0.1;

/// Default approve gas limit
pub const DEFAULT_APPROVE_GAS: u64 = 50_000;

/// Default swap gas limit
pub const DEFAULT_SWAP_GAS: u64 = 200_000;

/// Approve function signature
pub const APPROVE_FN_SIGNATURE: &str = "approve(address,uint256)";

/// Null address
pub const NULL_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Has executed flag
pub const HAS_EXECUTED: &str = "has_executed";

/// Default Redis host
pub const DEFAULT_REDIS_HOST: &str = "127.0.0.1:42044";

/// Default heartbeat URL
pub const DEFAULT_HEARTBEAT_URL: &str = "http://localhost:8080";

/// Default heartbeat delay
pub const HEARTBEAT_DELAY: u64 = 10;

/// Default config path
pub const DEFAULT_CONFIG_PATH: &str = "config/mmc.mainnet.eth-usdc.toml";
