// main.rs

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter, Set, Statement};
use serde_json::json;

use crate::{
    entity::{configuration, instance},
    types::{config::MoniEnvConfig, moni::ParsedMessage},
};

// The whole database URL string follows the following format:
// "protocol://username:password@host:port/database"
// We put the database name (that last bit) in a separate variable simply for convenience.

pub async fn connect(env: MoniEnvConfig) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(env.database_url.clone()).await?;
    match db.get_database_backend() {
        DbBackend::Postgres => {
            db.execute(Statement::from_string(db.get_database_backend(), format!("DROP DATABASE IF EXISTS \"{}\";", env.database_url.clone())))
                .await?;
            db.execute(Statement::from_string(db.get_database_backend(), format!("CREATE DATABASE \"{}\";", env.database_url.clone())))
                .await?;
            tracing::info!("🐘 Connecting to Neon");
            // tracing::info!("🐘 Connecting to Neon at {}", endpoint);
            Database::connect(&env.database_url).await
        }
        _ => {
            panic!("Neon: Unsupported database backend");
        }
    }
}

/// Handle different message types (from Redis pub-sub, to then push to DB)
pub async fn handle(msg: &ParsedMessage, env: MoniEnvConfig) {
    match msg {
        ParsedMessage::NewInstance(msg) => {
            tracing::info!("New instance msg received => new instance deployed. Config identifier: {}", msg.config.identifier(),);
            tracing::info!(" - Keccak256: {}", msg.config.keccak());
            // Current configuration in DB
            let db = connect(env.clone()).await.unwrap();
            // let config = configuration::Entity::find().filter(configuration::Column::Hash.eq(msg.config.keccak())).one(&db).await.unwrap();

            let cfgs = pull::configurations(&db).await.unwrap();
            tracing::info!(" - NewInstance: Found {} configurations in DB", cfgs.len());
            for cfg in cfgs.iter() {
                tracing::info!(" - Configuration: {}", cfg.hash);
            }
        }
        ParsedMessage::TradeEvent(msg) => {
            tracing::info!("Trade event: {}", msg.id);
            // TODO: Add logic to handle trade events
        }
        ParsedMessage::Unknown(data) => {
            tracing::warn!("Unknown message type: {:?}", data);
        }
    }
}

pub mod create {

    use crate::{
        entity::{configuration, trade},
        types::config::MarketMakerConfig,
    };

    use super::*;

    pub async fn configuration(db: &DatabaseConnection, mmc: MarketMakerConfig) -> Result<configuration::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let model = configuration::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            values: Set(config),
            hash: Set(mmc.keccak()),
            chain_id: Set(mmc.chain_id as i32),
            base_token_address: Set(mmc.base_token_address),
            quote_token_address: Set(mmc.quote_token_address),
            ..Default::default()
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("🐘 Inserted 'configuration' succeeded: {}", inserted.id);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("🐘 Error inserting: {}", err);
                Err(err)
            }
        }
    }

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

pub mod pull {
    use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter};

    use crate::entity::{configuration, instance, trade};

    pub async fn instances(db: &DatabaseConnection) -> Result<Vec<instance::Model>, sea_orm::DbErr> {
        let models = instance::Entity::find().all(db).await?;
        Ok(models)
    }

    pub async fn configurations(db: &DatabaseConnection) -> Result<Vec<configuration::Model>, sea_orm::DbErr> {
        let models = configuration::Entity::find().all(db).await?;
        Ok(models)
    }
}
