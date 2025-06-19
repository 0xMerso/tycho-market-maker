use crate::utils::r#static::CHANNEL_REDIS;

use std::{thread, time::Duration};

use redis::Commands;

pub async fn publish() {
    match crate::data::helpers::copubsub() {
        Ok(client) => match client.get_connection() {
            Ok(mut conn) => {
                for _i in 0..50 {
                    let msg = format!("this is test {:?} ", _i);
                    match conn.publish::<&str, &str, ()>(CHANNEL_REDIS, &msg) {
                        Ok(_) => {
                            println!("message has been send {:?}", msg)
                        }
                        Err(e) => {
                            println!("Publish message error {:?}", e.to_string())
                        }
                    }

                    thread::sleep(Duration::from_secs(1));
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
