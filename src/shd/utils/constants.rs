//! Application constants and configuration values

use std::sync::atomic::AtomicBool;

/// Redis channel for pub/sub communication
pub const CHANNEL_REDIS: &str = "tycho_market_maker";

/// Restart delay in seconds
pub const RESTART: u64 = 60;

/// Basis point denominator (10000 = 100%)
pub const BASIS_POINT_DENO: f64 = 10_000.0;

/// Price move threshold
pub const PRICE_MOVE_THRESHOLD: f64 = 0.5;

/// Add TVL threshold (minimum TVL for components to be monitored)
pub const ADD_TVL_THRESHOLD: f64 = 100.0;

/// Share pool balance swap basis points
pub const SHARE_POOL_BAL_SWAP_BPS: f64 = 0.1;

/// Default approve gas limit
pub const DEFAULT_APPROVE_GAS: u64 = 75_000;

/// Default swap gas limit
pub const DEFAULT_SWAP_GAS: u64 = 300_000;

/// Min amount worth USD to swap
pub const MIN_AMOUNT_WORTH_USD: f64 = 10.0;

/// Approve function signature
pub const APPROVE_FN_SIGNATURE: &str = "approve(address,uint256)";

/// Null address
pub const NULL_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Has executed flag
pub static HAS_EXECUTED: AtomicBool = AtomicBool::new(false);

/// Default Redis host
pub const DEFAULT_REDIS_HOST: &str = "127.0.0.1:42044";

/// Default heartbeat delay
pub const HEARTBEAT_DELAY: u64 = 300;

/// Optimization constants
pub const OPTI_TOLERANCE: f64 = 0.0001; // Stop when change is less than 0.01%
pub const OPTI_MAX_ITERATIONS: usize = 20;

/// Pool price validation constants
pub const MAX_POOL_PRICE_DEVIATION_PCT: f64 = 5.0; // Maximum allowed price deviation from reference (5%)
pub const PERCENT_MULTIPLIER: f64 = 100.0; // Multiplier to convert decimal to percentage
