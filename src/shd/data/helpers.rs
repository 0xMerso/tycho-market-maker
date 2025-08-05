#![allow(unused)] // silence unused warnings while exploring (to comment out)

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, time::Duration};
use tokio::time::sleep;

use redis::{
    aio::MultiplexedConnection,
    from_redis_value,
    streams::{StreamRangeReply, StreamReadOptions, StreamReadReply},
    AsyncCommands, Client, RedisError,
};

use crate::types::misc::StreamState;

/// =============================================================================
/// @function: ping
/// @description: Tests Redis connection by sending a PING command
/// @behavior: Sends PING to Redis server and panics if connection fails
/// =============================================================================
pub async fn ping() {
    let co = connect().await;
    match co {
        Ok(mut co) => {
            let pong: redis::RedisResult<String> = redis::cmd("PING").query_async(&mut co).await;
            match pong {
                Ok(pong) => {
                    tracing::debug!("📕 Redis Ping Good");
                }
                Err(e) => {
                    panic!("Redis PING Error: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("Redis PING Error: {}", e);
        }
    }
}

/// =============================================================================
/// @function: connect
/// @description: Establishes an async multiplexed connection to Redis server
/// @behavior: Reads REDIS_HOST from environment or uses default localhost:42044
/// =============================================================================
pub async fn connect() -> Result<MultiplexedConnection, RedisError> {
    let endpoint = std::env::var("REDIS_HOST"); // Contain port too
    let endpoint = match endpoint {
        Ok(endpoint) => endpoint,
        Err(_) => "127.0.0.1:42044".to_string(), // ! Default to update ?
    };
    let endpoint = format!("redis://{}", endpoint);
    // log::info!("Redis endpoint: {}", endpoint);
    let client = Client::open(endpoint);
    match client {
        Ok(client) => client.get_multiplexed_tokio_connection().await,
        Err(e) => {
            tracing::error!("Redis Client Error: {}", e);
            Err(e)
        }
    }
}

/// =============================================================================
/// @function: pubsub
/// @description: Creates a Redis client for pub/sub operations
/// @behavior: Reads REDIS_HOST from environment or uses default localhost:42044
/// =============================================================================
pub fn pubsub() -> Result<redis::Client, RedisError> {
    let endpoint = std::env::var("REDIS_HOST"); // Contain port too
    let endpoint = match endpoint {
        Ok(endpoint) => endpoint,
        Err(_) => "127.0.0.1:42044".to_string(), // ! Default to update ?
    };
    let endpoint = format!("redis://{}", endpoint);
    // tracing::info!("copubsub: endpoint: {}", endpoint);
    let client = Client::open(endpoint);
    match client {
        Ok(client) => Ok(client),
        Err(e) => {
            tracing::error!("Redis Client Error: {}", e);
            Err(e)
        }
    }
}

/// =============================================================================
/// @function: status
/// @description: Gets the database synchronization status for a given network
/// @param key: Network identifier key to check status for
/// @behavior: Maps numeric status values to StreamState enum variants
/// =============================================================================
pub async fn status(key: String) -> StreamState {
    let status = get::<u128>(key.as_str()).await;
    match status {
        Some(status) => match status {
            1 => StreamState::Down,
            2 => StreamState::Launching,
            3 => StreamState::Syncing,
            4 => StreamState::Running,
            _ => StreamState::Error,
        },
        None => StreamState::Error,
    }
}

/// =============================================================================
/// @function: wstatus
/// @description: Waits until database reaches 'Running' state for a network
/// @param key: Network identifier key to monitor
/// @param object: Description of what is being waited for (for logging)
/// @behavior: Polls status every 5 seconds until StreamState::Running is reached
/// =============================================================================
pub async fn wstatus(key: String, object: String) {
    let time = std::time::SystemTime::now();
    tracing::debug!("Waiting Redis Synchro");
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        let status = status(key.clone()).await;
        tracing::debug!("Waiting for '{object}'. Current status: {:?}", status);
        if let StreamState::Running = status {
            let elasped = time.elapsed().unwrap().as_millis();
            tracing::debug!("wstatus: redis db is ready. Took {} ms to sync", elasped);
            break;
        }
    }
}

/// =============================================================================
/// @function: delete
/// @description: Deletes a key-value pair from Redis
/// @param key: Redis key to delete
/// @behavior: Executes DEL command and logs errors if deletion fails
/// =============================================================================
pub async fn delete(key: &str) {
    let co = connect().await;
    match co {
        Ok(mut co) => {
            let deletion: redis::RedisResult<()> = redis::cmd("DEL").arg(key).query_async(&mut co).await;
            if let Err(err) = deletion {
                tracing::error!("Failed to delete JSON object with key '{}': {}", key, err);
            }
        }
        Err(e) => {
            tracing::error!("Redis connection error: {}", e);
        }
    }
}

/// =============================================================================
/// @function: set
/// @description: Stores a JSON-serialized object in Redis
/// @param key: Redis key to store value under
/// @param data: Generic serializable data to store
/// @behavior: Serializes data to JSON and stores using SET command
/// =============================================================================
pub async fn set<T: Serialize>(key: &str, data: T) {
    let data = serde_json::to_string(&data);
    match data {
        Ok(data) => {
            let co = connect().await;
            // let client = Client::open("redis://redis/");
            match co {
                Ok(mut co) => {
                    let result: redis::RedisResult<()> = redis::cmd("SET").arg(key).arg(data.clone()).query_async(&mut co).await;
                    if let Err(err) = result {
                        tracing::error!("📕 Failed to set value for key '{}': {}", key, err);
                    }
                }

                Err(e) => {
                    tracing::error!("📕 Redis connection error: {}", e);
                }
            }
        }
        Err(err) => {
            tracing::error!("📕 Failed to serialize JSON object: {}", err);
        }
    }
}

/// =============================================================================
/// @function: get
/// @description: Retrieves and deserializes a JSON object from Redis
/// @param key: Redis key to retrieve value from
/// @behavior: Fetches string value and deserializes to type T, returns None on error
/// =============================================================================
pub async fn get<T: Serialize + DeserializeOwned>(key: &str) -> Option<T> {
    let time = std::time::SystemTime::now();
    let co = connect().await;
    match co {
        Ok(mut co) => {
            let result: redis::RedisResult<String> = redis::cmd("GET").arg(key).query_async(&mut co).await;
            match result {
                Ok(value) => {
                    let elasped = time.elapsed().unwrap().as_millis();
                    match serde_json::from_str(&value) {
                        Ok(value) => {
                            // log::info!("📕 Get succeeded for key '{}'. Elapsed: {}ms", key, elasped);
                            Some(value)
                        }
                        Err(err) => {
                            tracing::error!("📕 Failed to deserialize JSON object: {}", err);
                            None
                        }
                    }
                }
                Err(err) => {
                    // log::error!("📕 Failed to get value for key '{}': {}", key, err);
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!("📕 Redis connection error: {}", e);
            None
        }
    }
}
