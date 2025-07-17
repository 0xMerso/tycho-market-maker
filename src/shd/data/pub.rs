use crate::types::moni::{MessageType, NewInstanceMessage, NewPricesMessage, NewTradeMessage, RedisMessage};
use crate::utils::constants::CHANNEL_REDIS;

use redis::Commands;
use serde::Serialize;
use serde_json;

/// Generic function to publish any serializable message to Redis pubsub
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

pub fn ping() -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::Ping,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(()).unwrap(),
    };
    publish(&message)
}

/// Publish a new instance launch event (for market_maker instances)
pub fn instance(msg: NewInstanceMessage) -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::NewInstance,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message)
}

/// Publish a trade event (flexible version that doesn't require database trade model)
pub fn prices(msg: NewPricesMessage) -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::NewPrices,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message)
}

/// Publish a trade event (flexible version that doesn't require database trade model)
pub fn trade(msg: NewTradeMessage) -> Result<(), String> {
    let message = RedisMessage {
        message: MessageType::NewTrade,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message)
}
