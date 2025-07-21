use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::types::{
    config::MarketMakerConfig,
    maker::{ComponentPriceData, ExecutedPayload},
};

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
    pub config: MarketMakerConfig, // Contain the whole data to be stored in DB
    pub identifier: String,
    pub commit: String,
}

/// New price message (simplified)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewPricesMessage {
    pub identifier: String,
    pub reference_price: f64,
    pub components: Vec<ComponentPriceData>,
    pub block: u64,
}

/// Trade event message (simplified)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewTradeMessage {
    pub identifier: String,
    pub block: u64,
    pub payload: Option<ExecutedPayload>,
}

/// Parsed message content
#[derive(Debug, Clone)]
pub enum ParsedMessage {
    NewInstance(NewInstanceMessage),
    NewPrices(NewPricesMessage),
    NewTrade(NewTradeMessage),
    Ping,
    Unknown(Value),
}

/// Message types for Redis pub/sub communication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "new_instance")]
    NewInstance,
    #[serde(rename = "new_trade")]
    NewTrade,
    #[serde(rename = "new_prices")]
    NewPrices,
}
