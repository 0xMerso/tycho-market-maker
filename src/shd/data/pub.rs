use crate::entity::{instance::Model as Instance, trade::Model as Trade};
use crate::types::config::MarketMakerConfig;
use crate::types::moni::{MessageType, NewInstanceMessage, NewPricesMessage, NewTradeMessage, RedisMessage};
use crate::utils::r#static::CHANNEL_REDIS;

use redis::Commands;
use serde::Serialize;
use serde_json;

/// Generic function to publish any serializable message to Redis pubsub
pub fn publish<T: Serialize>(event: &T) {
    let time = std::time::SystemTime::now();
    match crate::data::helpers::copubsub() {
        Ok(client) => match client.get_connection() {
            Ok(mut conn) => {
                let msg = serde_json::to_string(event).unwrap();
                match conn.publish::<&str, &str, ()>(CHANNEL_REDIS, &msg) {
                    Ok(_) => {
                        let elasped = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Message has been sent (of size: {}) | Took {} ms", msg.len(), elasped);
                    }
                    Err(e) => {
                        tracing::debug!("Publish message error {:?}", e.to_string())
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error while getting connection: {}", e.to_string());
            }
        },
        Err(e) => {
            tracing::error!("Error while getting connection: {}", e.to_string());
        }
    }
}

/// Publish a new instance launch event (for mk2 instances)
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
