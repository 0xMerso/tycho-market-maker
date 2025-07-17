use std::time::Duration;

use crate::{
    types::config::{EnvConfig, MarketMakerConfig},
    utils::constants::HEARTBEAT_DELAY,
};

/// Send a heartbeat 200 Get
pub async fn alive(endpoint: String) -> bool {
    let client = reqwest::Client::new();

    match client.get(endpoint.clone()).send().await {
        Ok(res) => {
            tracing::debug!("Heartbeat Success: {}", res.status());
            true
        }
        Err(e) => {
            tracing::error!("Heartbeat Error on {}: {}", endpoint, e);
            false
        }
    }
}

/// Conditional heartbeat, with a dedicated task. Not used for now.
/// @param _mmc: Market maker configuration (unused but kept for future use)
/// @param env: Environment configuration containing testing mode and heartbeat endpoint
pub async fn heartbeats(_mmc: MarketMakerConfig, env: EnvConfig) {
    if env.testing {
        tracing::info!("Testing mode, heartbeat task not spawned.");
        return;
    }
    tracing::info!("Spawning heartbeat task.");
    tokio::spawn(async move {
        let mut hb = tokio::time::interval(Duration::from_secs(HEARTBEAT_DELAY / 2));
        loop {
            hb.tick().await;
            tracing::debug!("Heartbeat tick. Endpoint: {}", env.heartbeat);
        }
    });
}
