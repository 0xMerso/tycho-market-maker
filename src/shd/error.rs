//! Error Handling Module
//!
//! Centralized error handling for the market maker application.
//! This module defines the main error types and provides a unified error handling
//! system for configuration, database, network, and execution errors.
use thiserror::Error;

/// Main error type for market maker operations.
#[derive(Error, Debug)]
pub enum MarketMakerError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Token not found: {0}")]
    TokenNotFound(String),

    #[error("Price feed error: {0}")]
    PriceFeed(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Environment variable not found: {0}")]
    EnvVar(String),
}

/// Type alias for Result with MarketMakerError.
pub type Result<T> = std::result::Result<T, MarketMakerError>;

impl From<std::env::VarError> for MarketMakerError {
    fn from(err: std::env::VarError) -> Self {
        MarketMakerError::EnvVar(err.to_string())
    }
}
