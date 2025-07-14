use thiserror::Error;

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

pub type Result<T> = std::result::Result<T, MarketMakerError>;

impl From<std::env::VarError> for MarketMakerError {
    fn from(err: std::env::VarError) -> Self {
        MarketMakerError::EnvVar(err.to_string())
    }
}
