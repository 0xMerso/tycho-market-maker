use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    process::Command,
};

use serde::{Serialize, de::DeserializeOwned};

/// Get the current Git commit hash
/// Make sure, if running within Docker, git is installed in the Dockerfile
pub fn commit() -> Option<String> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // tracing::debug!("♻️  Commit: {}", commit);
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

/**
 * Read a file and return a Vec<T> where T is a deserializable type
 */
pub fn read<T: DeserializeOwned>(file: &str) -> Vec<T> {
    let mut f = File::open(file).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();
    let db: Vec<T> = serde_json::from_str(&buffer).unwrap();
    db
}

/**
 * Write output to file
 * bot/src/network/snapshots/base.aave.json
 */
pub fn save<T: Serialize>(output: Vec<T>, file: &str) {
    // log::info!("Saving to file: {}", file);
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(file).expect("Failed to open or create file");
    let json = serde_json::to_string(&output).expect("Failed to serialize JSON");
    file.write_all(json.as_bytes()).expect("Failed to write to file");
    file.write_all(b"\n").expect("Failed to write newline to file");
    file.flush().expect("Failed to flush file");
}
