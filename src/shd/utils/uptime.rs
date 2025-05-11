use std::time::Duration;

use crate::types::config::{EnvConfig, MarketMakerConfig};

use super::r#static::HEARTBEAT_DELAY;

/// Send a heartbeat 200 Get
pub async fn alive(endpoint: String) -> bool {
    let client = reqwest::Client::new();
    match client.get(endpoint.clone()).send().await {
        Ok(res) => {
            tracing::debug!("Hearbeat Success: {}", res.status());
            true
        }
        Err(e) => {
            tracing::error!("Hearbeat Error on {}: {}", endpoint, e);
            false
        }
    }
}

/// Conditional heartbeat, with a dedicated task. Not used for now.
/// 1. Fetch Redis data size > 0
/// 2. Assert Network status latest > 0
pub async fn hearbeats(mmc: MarketMakerConfig, env: EnvConfig) {
    if env.testing {
        tracing::info!("Testing mode, heartbeat task not spawned.");
        return;
    } else {
        tracing::info!("Spawning heartbeat task.");
    }
    tokio::spawn(async move {
        let mut hb = tokio::time::interval(Duration::from_secs(HEARTBEAT_DELAY / 2));
        loop {
            hb.tick().await;
            tracing::debug!("Heartbeat tick. Endpoint: {}", env.heartbeat);
            // ToDo
        }
    });
}
