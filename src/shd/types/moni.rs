use serde::{Deserialize, Serialize};

/// Struct representing a trade event to be sent to monitoring
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct TradeEvent {
    pub event_type: String,      // e.g. "trade_attempt", "trade_success", "trade_fail"
    pub meta: String,            // freeform metadata/context
    pub block_time: u64,         // block timestamp or system time
    pub tx_hash: Option<String>, // transaction hash if available
    pub details: Option<String>, // any extra details (JSON, etc)
}

use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: i64,
    pub created_at: i64, // epoch seconds
    pub updated_at: i64,
    pub deleted_at: i64,
    pub pre_trade_context: Value,  // JSON blob
    pub details: Value,            // JSON blob
    pub post_trade_context: Value, // JSON blob
    pub log_id: i64,               // FK → Log
    pub bot_id: i64,               // FK → Bot
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: i64,
    pub reference: Value, // JSON blob
    pub pools: Value,     // JSON blob
    pub log_id: i64,      // FK → Log
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bot {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: i64,
    pub config: Value, // JSON blob
}
