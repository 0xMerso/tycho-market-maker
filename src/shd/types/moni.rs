use serde::{Deserialize, Serialize};

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

/// ======================================================================================= Events PUB/SUB =====================================================================================================
/// Redis message structure

/// Base message structure for all Redis messages
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisMessage {
    pub message: MessageType,
    pub timestamp: u64,
    pub data: Value,
}

/// New instance deployment message (simplified)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewInstanceMessage {
    pub instance_id: String,
    pub network: String,
}

/// Trade event message (simplified)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeEventMessage {
    pub instance_id: String,
    pub tx_hash: String,
    pub status: String,
}

/// Parsed message content
#[derive(Debug, Clone)]
pub enum ParsedMessage {
    NewInstance(NewInstanceMessage),
    TradeEvent(TradeEventMessage),
    Unknown(Value),
}

/// Message types for Redis pub/sub communication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    #[serde(rename = "new_instance")]
    NewInstance,
    #[serde(rename = "trade_event")]
    TradeEvent,
}
