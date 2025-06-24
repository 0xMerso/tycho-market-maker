use crate::types::config::MoniEnvConfig;
use crate::types::moni::{MessageType, NewInstanceMessage, ParsedMessage, RedisMessage, TradeEventMessage};
use crate::utils::r#static::CHANNEL_REDIS;
use serde_json;

/// Parse a JSON string into a ParsedMessage
pub fn parse(value: &str) -> Result<ParsedMessage, String> {
    let rdmsg: RedisMessage = serde_json::from_str(value).map_err(|e| format!("Failed to parse Redis message: {}", e))?;

    match rdmsg.message {
        MessageType::NewInstance => {
            let msg: NewInstanceMessage = serde_json::from_value(rdmsg.data).map_err(|e| format!("Failed to parse NewInstance message: {}", e))?;
            Ok(ParsedMessage::NewInstance(msg))
        }
        MessageType::TradeEvent => {
            let msg: TradeEventMessage = serde_json::from_value(rdmsg.data).map_err(|e| format!("Failed to parse TradeEvent message: {}", e))?;
            Ok(ParsedMessage::TradeEvent(msg))
        }
    }
}

/// Listen to the Redis channel and parse different message types
pub async fn listen(env: MoniEnvConfig) {
    match crate::data::helpers::copubsub() {
        Ok(client) => match client.get_connection() {
            Ok(mut conn) => {
                let mut pubsub = conn.as_pubsub();
                tracing::info!("Redis pub-sub channel: '{}'", CHANNEL_REDIS);
                match pubsub.subscribe(CHANNEL_REDIS) {
                    Ok(_) => loop {
                        match pubsub.get_message() {
                            Ok(msg) => match msg.get_payload::<String>() {
                                Ok(payload) => {
                                    tracing::debug!("New message received (of size: {})", payload.len());
                                    match parse(&payload) {
                                        Ok(pm) => {
                                            crate::data::neon::handle(&pm, env.clone()).await;
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to parse message: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error while getting payload: {}", e.to_string());
                                }
                            },
                            Err(e) => {
                                tracing::error!("Error: {}", e.to_string());
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!("{}", e.to_string());
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
