use crate::types::config::MoniEnvConfig;
use crate::types::moni::{MessageType, NewInstanceMessage, NewPricesMessage, NewTradeMessage, ParsedMessage, RedisMessage};
use crate::utils::constants::CHANNEL_REDIS;
use serde_json;

///   =============================================================================
/// @function: parse
/// @description: Parse a JSON string from Redis into a strongly-typed ParsedMessage
/// @param value: JSON string received from Redis pub/sub channel
/// @behavior: Deserializes the JSON and returns appropriate ParsedMessage variant based on MessageType
///   =============================================================================
pub fn parse(value: &str) -> Result<ParsedMessage, String> {
    let rdmsg: RedisMessage = serde_json::from_str(value).map_err(|e| format!("Failed to parse Redis message: {}", e))?;

    match rdmsg.message {
        MessageType::Ping => Ok(ParsedMessage::Ping),
        MessageType::NewInstance => {
            let msg: NewInstanceMessage = serde_json::from_value(rdmsg.data).map_err(|e| format!("Failed to parse NewInstance message: {}", e))?;
            Ok(ParsedMessage::NewInstance(msg))
        }
        MessageType::NewTrade => {
            let msg: NewTradeMessage = serde_json::from_value(rdmsg.data).map_err(|e| format!("Failed to parse NewTrade message: {}", e))?;
            Ok(ParsedMessage::NewTrade(msg))
        }
        MessageType::NewPrices => {
            let msg: NewPricesMessage = serde_json::from_value(rdmsg.data).map_err(|e| format!("Failed to parse NewPrices message: {}", e))?;
            Ok(ParsedMessage::NewPrices(msg))
        }
    }
}

///   =============================================================================
/// @function: listen
/// @description: Continuously listens to Redis pub/sub channel for market maker events
/// @param env: Monitoring environment configuration containing connection details
/// @behavior: Subscribes to CHANNEL_REDIS, processes incoming messages, and forwards them to Neon database handler
///   =============================================================================
pub async fn listen(env: MoniEnvConfig) {
    let Ok(client) = crate::data::helpers::pubsub() else {
        tracing::error!("Error while getting connection 3");
        return;
    };

    let Ok(mut conn) = client.get_connection() else {
        tracing::error!("Error while getting connection 4");
        return;
    };

    let mut pubsub = conn.as_pubsub();
    tracing::info!("Redis pub-sub channel: '{}'", CHANNEL_REDIS);

    let Ok(_) = pubsub.subscribe(CHANNEL_REDIS) else {
        tracing::error!("Failed to subscribe to channel");
        return;
    };

    loop {
        let Ok(msg) = pubsub.get_message() else {
            tracing::error!("Error getting message");
            continue;
        };

        let Ok(payload) = msg.get_payload::<String>() else {
            tracing::error!("Error while getting payload");
            continue;
        };

        // tracing::trace!("New message received (size: {})", payload.len());

        match parse(&payload) {
            Ok(parsed_message) => {
                crate::data::neon::handle(&parsed_message, env.clone()).await;
            }
            Err(e) => {
                tracing::error!("Failed to parse message: {}", e);
            }
        }

        // Sleep for 100ms ?
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}
