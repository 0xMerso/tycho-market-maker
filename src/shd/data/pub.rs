use crate::entity::{instance::Model as Instance, trade::Model as Trade};
use crate::types::config::MarketMakerConfig;
use crate::types::moni::{MessageType, NewInstanceMessage, NewPricesMessage, NewTradeMessage, RedisMessage};
use crate::utils::r#static::CHANNEL_REDIS;

use redis::Commands;
use serde::Serialize;
use serde_json;

/// Generic function to publish any serializable message to Redis pubsub
pub fn publish<T: Serialize>(event: &T) {
    let start_time = std::time::SystemTime::now();

    let Ok(client) = crate::data::helpers::pubsub() else {
        tracing::error!("Error while getting connection");
        return;
    };

    let Ok(mut conn) = client.get_connection() else {
        tracing::error!("Error while getting connection");
        return;
    };

    let Ok(msg) = serde_json::to_string(event) else {
        tracing::error!("Failed to serialize message");
        return;
    };

    match conn.publish::<&str, &str, ()>(CHANNEL_REDIS, &msg) {
        Ok(_) => {
            let elapsed = start_time.elapsed().unwrap_or_default().as_millis();
            tracing::debug!("Message has been sent (of size: {}) | Took {} ms", msg.len(), elapsed);
        }
        Err(e) => {
            tracing::debug!("Publish message error {:?}", e.to_string())
        }
    }
}

/// Publish a new instance launch event (for market_maker instances)
pub fn instance(msg: NewInstanceMessage) {
    let message = RedisMessage {
        message: MessageType::NewInstance,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message);
}

/// Publish a trade event (flexible version that doesn't require database trade model)
pub fn prices(msg: NewPricesMessage) {
    let message = RedisMessage {
        message: MessageType::NewPrices,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message);
}

/// Publish a trade event (flexible version that doesn't require database trade model)
pub fn trade(msg: NewTradeMessage) {
    let message = RedisMessage {
        message: MessageType::NewTrade,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        data: serde_json::to_value(msg).unwrap(),
    };
    publish(&message);
}
