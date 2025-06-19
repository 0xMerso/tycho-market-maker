use serde::{Deserialize, Serialize};

/// Struct representing a trade event to be sent to monitoring
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TradeEvent {
    pub event_type: String,      // e.g. "trade_attempt", "trade_success", "trade_fail"
    pub meta: String,            // freeform metadata/context
    pub block_time: u64,         // block timestamp or system time
    pub tx_hash: Option<String>, // transaction hash if available
    pub details: Option<String>, // any extra details (JSON, etc)
}
