use crate::utils::r#static::CHANNEL_REDIS;

use redis::Commands;
use serde::Serialize;
use serde_json;

pub fn publish<T: Serialize>(event: &T) {
    let time = std::time::SystemTime::now();
    match crate::data::helpers::copubsub() {
        Ok(client) => match client.get_connection() {
            Ok(mut conn) => {
                let msg = serde_json::to_string(event).unwrap();
                match conn.publish::<&str, &str, ()>(CHANNEL_REDIS, &msg) {
                    Ok(_) => {
                        let elasped = time.elapsed().unwrap_or_default().as_millis();
                        tracing::debug!("Message has been sent {:?}. Took {} ms", msg, elasped);
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
