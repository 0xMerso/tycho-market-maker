// main.rs

use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Set, Statement};
use serde_json::json;

use crate::{entity::instance, types::config::MoniEnvConfig};

// The whole database URL string follows the following format:
// "protocol://username:password@host:port/database"
// We put the database name (that last bit) in a separate variable simply for convenience.

pub async fn connect(env: MoniEnvConfig) -> Result<DatabaseConnection, DbErr> {
    tracing::info!("Connecting to Neon");
    let db = Database::connect(env.database_url.clone()).await?;
    match db.get_database_backend() {
        DbBackend::Postgres => {
            db.execute(Statement::from_string(db.get_database_backend(), format!("DROP DATABASE IF EXISTS \"{}\";", env.database_url.clone())))
                .await?;
            db.execute(Statement::from_string(db.get_database_backend(), format!("CREATE DATABASE \"{}\";", env.database_url.clone())))
                .await?;
            tracing::info!("🐘 Connecting to Neon at {}", env.database_url);
            Database::connect(&env.database_url).await
        }
        _ => {
            panic!("Unsupported database backend");
        }
    }
}

pub mod pull {
    use sea_orm::{DatabaseConnection, EntityTrait};

    use crate::entity::instance;

    pub async fn instances(db: &DatabaseConnection) -> Result<Vec<instance::Model>, sea_orm::DbErr> {
        let models = instance::Entity::find().all(db).await?;
        Ok(models)
    }
}

pub mod create {

    use crate::{entity::trade, types::config::MarketMakerConfig};

    use super::*;

    /// Insert a new Bot and return its full Model (with id, timestamps, …)
    pub async fn instance(db: &DatabaseConnection, mmc: MarketMakerConfig) -> Result<instance::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let model = instance::ActiveModel {
            config: Set(config),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("🐘 Inserted 'instance' succeeded: {}", inserted.id);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("🐘 Error inserting: {}", err);
                Err(err)
            }
        }
    }

    /// Insert a new Bot and return its full Model (with id, timestamps, …)
    pub async fn trade(db: &DatabaseConnection, mmc: MarketMakerConfig) -> Result<trade::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let model = trade::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            // deleted_at: Set(None),
            ..Default::default()
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("🐘 Inserted 'trade' succeeded: {}", inserted.id);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("🐘 Error inserting: {}", err);
                Err(err)
            }
        }
    }
}
