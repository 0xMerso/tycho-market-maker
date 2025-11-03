use crate::types::moni::{MessageType, NewInstanceMessage, NewPricesMessage, NewTradeMessage, RedisMessage};
use crate::utils::constants::CHANNEL_REDIS;

use redis::Commands;
use serde::Serialize;
use serde_json;

///   =============================================================================
/// @function: publish
/// @description: Generic function to publish any serializable message to Redis pubsub
/// @param event: Any serializable event to publish to Redis channel
/// @behavior: Serializes the event to JSON and publishes it to CHANNEL_REDIS
///   =============================================================================
pub fn publish<T: Serialize>(event: &T) -> Result<(), String> {
    let start_time = std::time::SystemTime::now();

    let Ok(client) = crate::data::helpers::pubsub() else {
        tracing::error!("Error while getting connection 1");
        return Err("Error while getting connection 1".to_string());
    };

    let Ok(mut conn) = client.get_connection() else {
        tracing::error!("Error while getting connection 2");
        return Err("Error while getting connection 2".to_string());
    };

    let Ok(msg) = serde_json::to_string(event) else {
        tracing::error!("Failed to serialize message");
        return Err("Failed to serialize message".to_string());
    };

    match conn.publish::<&str, &str, ()>(CHANNEL_REDIS, &msg) {
        Ok(_) => {
            let _elapsed = start_time.elapsed().unwrap_or_default().as_millis();
            // tracing::debug!("Message has been sent (of size: {}) | Took {} ms", msg.len(), elapsed);
            Ok(())
        }
        Err(e) => {
            tracing::debug!("Publish message error {:?}", e.to_string());
            Err(e.to_string())
        }
    }
}

///   =============================================================================
/// @function: ping
/// @description: Publishes a ping message to verify Redis connectivity and health
/// @behavior: Creates a Ping message type and publishes it to the Redis channel
///   =============================================================================
pub fn ping() -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::Ping,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(()).unwrap(),
    };
    publish(&message)
}

///   =============================================================================
/// @function: instance
/// @description: Publishes a new market maker instance creation event
/// @param msg: NewInstanceMessage containing instance configuration and metadata
/// @behavior: Wraps the message in RedisMessage and publishes to monitoring system
///   =============================================================================
pub fn instance(msg: NewInstanceMessage) -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::NewInstance,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message)
}

///   =============================================================================
/// @function: prices
/// @description: Publishes price update events from the market maker
/// @param msg: NewPricesMessage containing latest price information
/// @behavior: Wraps price data in RedisMessage and publishes for monitoring
///   =============================================================================
pub fn prices(msg: NewPricesMessage) -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::NewPrices,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message)
}

///   =============================================================================
/// @function: trade
/// @description: Publishes trade execution events from the market maker
/// @param msg: NewTradeMessage containing trade details and execution information
/// @behavior: Wraps trade data in RedisMessage and publishes for monitoring and analysis
///   =============================================================================
pub fn trade(msg: NewTradeMessage) -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::NewTrade,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message)
}
