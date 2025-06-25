use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::types::{
    config::MarketMakerConfig,
    maker::{Inventory, MarketContext},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub hash: i64,
    pub context: Value,
}

// In theory, snapshot should be made at a network block time frequency, but it can slightly vary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakerSnapshot {
    pub block: u64,
    pub timestamp: u64,
    pub reference_price: f64,
    pub market_context: MarketContext,
    pub components: Vec<ComponentPriceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPriceData {
    pub address: String,
    pub r#type: String,
    pub price: f64,
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
    pub config: MarketMakerConfig, // Contain the whole data to be stored in DB
    pub commit: String,
}

/// New price message (simplified)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewPricesMessage {
    pub config: MarketMakerConfig, // Add config to link to instance
    pub instance_hash: String,     // Add instance hash for precise linking
    pub reference_price: f64,
    pub components: Vec<ComponentPriceData>,
}

/// Trade event message (simplified)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewTradeMessage {
    pub config: MarketMakerConfig,
    pub instance_hash: String, // Add instance hash for precise linking
    pub trade: TradeData,
}

/// Parsed message content
#[derive(Debug, Clone)]
pub enum ParsedMessage {
    NewInstance(NewInstanceMessage),
    NewPrices(NewPricesMessage),
    NewIntent(NewTradeMessage),
    NewTrade(NewTradeMessage),
    Unknown(Value),
}

/// Message types for Redis pub/sub communication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    #[serde(rename = "new_instance")]
    NewInstance,
    #[serde(rename = "new_trade")]
    NewTrade,
    #[serde(rename = "new_prices")]
    NewPrices,
}
