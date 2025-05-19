// Static variables

pub static HEARTBEAT_DELAY: u64 = 300;
pub static RESTART: u64 = 60;
pub static ADD_TVL_THRESHOLD: f64 = 100.; // 50 iteration maximum to optimize allocation
pub static NULL_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub static BASIS_POINT_DENO: f64 = 10000.;
pub static COINGECKO_ETH_USD: &str = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
