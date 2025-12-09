//! Market Maker Monitoring Service
//!
//! Standalone monitoring service that listens to market maker events and stores them
//! in the database for analysis and tracking. Connects to Neon PostgreSQL, listens
//! to Redis pub/sub for market maker events, and provides real-time performance monitoring.
use shd::{types::config::MoniEnvConfig, utils::constants::CHANNEL_REDIS};
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Main entry point for the monitoring service.
///
/// Initializes logging, loads configuration, establishes database connection,
/// and starts listening to Redis pub/sub for market maker events.
#[tokio::main]
async fn main() {
    // Initialize logging with environment-based configuration
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_max_level(Level::TRACE).with_env_filter(filter).init();

    // Load monitor-specific environment configuration
    dotenv::from_filename("config/secrets/.env.monitor.global").ok();
    let env = MoniEnvConfig::new();
    env.print();

    // Log current commit for debugging
    let commit = shd::utils::misc::commit();
    tracing::info!("♻️  Monitor program commit: {:?}", commit);

    tracing::info!("Launching MM monitoring program | 🧪 Testing mode: {:?}", env.testing);

    // Initialize and test database connection
    tracing::info!("🐘 Init and test connection to Neon, Prisma, SeaORM, to PgSQL");

    // Establish database connection with error handling
    let Ok(db) = shd::data::neon::connect(env.clone()).await else {
        tracing::error!("Failed to connect to Neon database");
        return;
    };

    tracing::info!("🐘 Neon connected");

    // Validate database connectivity by fetching configurations
    match shd::data::neon::pull::configurations(&db).await {
        Ok(configurations) => {
            tracing::info!("🐘 Found {} configurations in DB", configurations.len());
        }
        Err(err) => {
            tracing::error!("Error fetching configurations from DB: {}", err);
            return;
        }
    }

    // Spawn heartbeat task
    shd::utils::uptime::heartbeats(env.testing, env.heartbeat.clone()).await;

    // Start listening to Redis pub/sub channel for market maker events
    tracing::info!("🐘 Starting infinite listening of the Redis pub-sub channel: {}, for MM events", CHANNEL_REDIS);
    shd::data::sub::listen(env.clone()).await;

    tracing::info!("Monitoring program finished");
}
