use std::process::Command;

/// Get the current Git commit hash
/// Make sure, if running within Docker, git is installed in the Dockerfile
pub fn commit() -> Option<String> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
                tracing::debug!("♻️  Commit: {}", commit);
                Some(commit)
            } else {
                let error_message = String::from_utf8_lossy(&output.stderr);
                tracing::debug!("♻️  Error status to get commit hash: {}", error_message);
                None
            }
        }
        Err(e) => {
            tracing::error!("Failed to exec git rev-parse: {}", e);
            None
        }
    }
}

/// Get an environment variable
pub fn get(key: &str) -> String {
    match std::env::var(key) {
        Ok(x) => x,
        Err(_) => {
            panic!("Environment variable not found: {}", key);
        }
    }
}
