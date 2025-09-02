use std::sync::atomic::AtomicBool;

// Core constants
pub static HEARTBEAT_DELAY: u64 = 300;
pub static RESTART: u64 = 60;
pub static ADD_TVL_THRESHOLD: f64 = 100.;
pub static NULL_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub static BASIS_POINT_DENO: f64 = 10000.;
pub static SHARE_POOL_BAL_SWAP_BPS: f64 = 10.;
pub static COINGECKO_ETH_USD: &str = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";

// Optimization constants
pub static OPTI_LOW_FACTOR: f64 = 1e-6;
pub static OPTI_HIGH_FACTOR: f64 = 1e5;
pub static OPTI_TOLERANCE: f64 = 0.0001; // Stop when change is less than 0.01%
pub static OPTI_MAX_ITERATIONS: usize = 20;

// Execution constants
pub static APPROVE_FN_SIGNATURE: &str = "approve(address,uint256)";
pub static DEFAULT_APPROVE_GAS: u64 = 100_000;
pub static DEFAULT_SWAP_GAS: u64 = 500_000;
pub static HAS_EXECUTED: AtomicBool = AtomicBool::new(false);

// Monitoring constants
pub static CHANNEL_REDIS: &str = "PubSub_Channel_Redis_MarketMaker";
pub static PRICE_MOVE_THRESHOLD: f64 = 1.0;
pub static PRICE_MOVE_DENO: f64 = 10_000.0;
