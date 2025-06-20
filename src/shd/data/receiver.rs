use crate::utils::r#static::CHANNEL_REDIS;

/// Listen to the Redis channel and print the messages
pub fn listen() {
    match crate::data::helpers::copubsub() {
        Ok(client) => match client.get_connection() {
            Ok(mut conn) => {
                let mut pubsub = conn.as_pubsub();
                tracing::info!("Redis pub-sub channel: '{}'", CHANNEL_REDIS);
                match pubsub.subscribe(CHANNEL_REDIS) {
                    Ok(_pubs) => loop {
                        match pubsub.get_message() {
                            Ok(msg) => match msg.get_payload::<String>() {
                                Ok(payload) => {
                                    tracing::info!("Message Received: {}. Pushing to DB.", payload.parse::<String>().unwrap());
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
