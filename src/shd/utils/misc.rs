//! Miscellaneous Utility Functions
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    process::Command,
};

use serde::{de::DeserializeOwned, Serialize};

/// Gets the current Git commit hash from the repository.
pub fn commit() -> Option<String> {
    let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() else {
        tracing::error!("Failed to exec git rev-parse");
        return None;
    };

    if output.status.success() {
        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(commit)
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("♻️  Error status to get commit hash: {}", error_message);
        None
    }
}

/// Retrieves an environment variable value, panics if not found.
pub fn get(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("Environment variable not found: {}", key))
}

/// Reads and deserializes a JSON file into a vector of type T.
pub fn read<T: DeserializeOwned>(file: &str) -> Vec<T> {
    let mut file = File::open(file).unwrap();
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();
    serde_json::from_str(&buffer).unwrap()
}

/// Serializes and saves a vector to a JSON file.
pub fn save<T: Serialize>(output: Vec<T>, file: &str) {
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(file).expect("Failed to open or create file");

    let json = serde_json::to_string(&output).expect("Failed to serialize JSON");
    file.write_all(json.as_bytes()).expect("Failed to write to file");
    file.write_all(b"\n").expect("Failed to write newline to file");
    file.flush().expect("Failed to flush file");
}
