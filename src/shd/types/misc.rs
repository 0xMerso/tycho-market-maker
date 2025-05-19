use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

/// Used to safely progress with Redis database
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StreamState {
    Down = 1,
    Launching = 2,
    Syncing = 3,
    Running = 4,
    Error = 5,
}

impl Display for StreamState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StreamState::Down => write!(f, "Down"),
            StreamState::Launching => write!(f, "Launching"),
            StreamState::Syncing => write!(f, "Syncing"),
            StreamState::Running => write!(f, "Running"),
            StreamState::Error => write!(f, "Error"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CoinGeckoResponse {
    pub ethereum: CryptoPrice,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CryptoPrice {
    pub usd: f64,
}
