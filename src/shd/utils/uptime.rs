use std::{process::Command, time::Duration};

use crate::utils::constants::HEARTBEAT_DELAY;

/// Sends HTTP GET heartbeat request to check endpoint health.
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

/// Retrieves the current Git commit hash.
pub fn ghead() -> Option<String> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output().expect("Failed to execute git command");
    if output.status.success() {
        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!("♻️  Commit: {}", commit);
        Some(commit)
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        tracing::info!("♻️  Failed to get Git Commit hash: {}", error_message);
        None
    }
}

/// Sends a heartbeat ping to a specified endpoint for monitoring.
pub async fn heartbeat(endpoint: String) {
    ghead();
    let client = reqwest::Client::new();
    let _res = match client.get(endpoint.clone()).send().await {
        Ok(res) => {
            tracing::info!("Hearbeat Success for {}: {}", endpoint.clone(), res.status());
            res
        }
        Err(e) => {
            tracing::error!("Hearbeat Error on {}: {}", endpoint, e);
            return;
        }
    };
}

/// Spawns background task for periodic heartbeat monitoring.
pub async fn heartbeats(testing: bool, heartbeat_endpoint: String) {
    if testing {
        tracing::info!("Testing mode, heartbeat task not spawned.");
        return;
    }
    tracing::info!("Spawning heartbeat task.");
    tokio::spawn(async move {
        let mut hb = tokio::time::interval(Duration::from_secs(HEARTBEAT_DELAY / 2));
        loop {
            hb.tick().await;
            heartbeat(heartbeat_endpoint.clone()).await;
            tracing::debug!("Heartbeat tick. Endpoint: {}", heartbeat_endpoint);
        }
    });
}
