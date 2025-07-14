use shd::{types::config::MoniEnvConfig, utils::constants::CHANNEL_REDIS};
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_max_level(Level::TRACE).with_env_filter(filter).init();

    dotenv::from_filename("config/.env.monitor.ex").ok();
    let env = MoniEnvConfig::new();
    env.print();

    let commit = shd::utils::misc::commit();
    tracing::info!("♻️  Monitor program commit: {:?}", commit);

    tracing::info!("Launching MM monitoring program | 🧪 Testing mode: {:?}", env.testing);

    tracing::info!("🐘 Init and test connection to Neon, Prisma, SeaORM, to PgSQL");

    // Need error handling
    let Ok(db) = shd::data::neon::connect(env.clone()).await else {
        tracing::error!("Failed to connect to Neon database");
        return;
    };

    tracing::info!("🐘 Neon connected");

    match shd::data::neon::pull::configurations(&db).await {
        Ok(configurations) => {
            tracing::info!("🐘 Found {} configurations in DB", configurations.len());
        }
        Err(err) => {
            tracing::error!("Error fetching configurations from DB: {}", err);
            tracing::error!("🐘 Make sure Neon has tables, etc. Exiting ...");
            return;
        }
    }

    tracing::info!("🐘 Starting infinite listening of the Redis pub-sub channel: {}, for MM events", CHANNEL_REDIS);
    shd::data::sub::listen(env.clone()).await;

    tracing::info!("Monitoring program finished");
}
