use std::sync::atomic::AtomicBool;

/// ======= Static variables =======

pub static HEARTBEAT_DELAY: u64 = 300;
pub static RESTART: u64 = 60;
pub static ADD_TVL_THRESHOLD: f64 = 100.; // 50 iteration maximum to optimize allocation
pub static NULL_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub static BASIS_POINT_DENO: f64 = 10000.;
pub static SHARE_POOL_BAL_SWAP_BPS: f64 = 10.; // 10 bps 
pub static COINGECKO_ETH_USD: &str = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";

/// --- Exec ---
pub static APPROVE_FN_SIGNATURE: &str = "approve(address,uint256)";
pub static DEFAULT_APPROVE_GAS: u64 = 100_000;
pub static DEFAULT_SWAP_GAS: u64 = 500_000;

pub static HAS_EXECUTED: AtomicBool = AtomicBool::new(false);

/// Monitoring
pub static CHANNEL_REDIS: &str = "PubSub_Channel_Redis_MarketMaker"; // Channel Redis MM

pub static PRICE_MOVE_THRESHOLD: f64 = 1.0; // 1/100 of 1 bps
pub static PRICE_MOVE_DENO: f64 = 1_000_000.0; // 1/100 of 1 bps denominator
