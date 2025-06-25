// main.rs

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter, Set, Statement};
use serde_json::json;

use crate::{
    entity::{configuration, instance, price},
    types::{
        config::{MarketMakerConfig, MoniEnvConfig},
        moni::ParsedMessage,
    },
};
use sea_orm::prelude::Uuid;

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
            tracing::trace!("NewInstance received with config identifier: {}", msg.config.identifier());
            tracing::trace!("Config Keccak256: {}", msg.config.keccak());
            // Current configuration in DB
            let db = connect(env.clone()).await.unwrap();
            // let config = configuration::Entity::find().filter(configuration::Column::Hash.eq(msg.config.keccak())).one(&db).await.unwrap();
            let cfgs = pull::configurations(&db).await.unwrap();
            let hash = msg.config.keccak().to_lowercase();
            match cfgs.iter().find(|cfg| cfg.hash.to_lowercase() == hash) {
                Some(cfg) => {
                    tracing::info!("Configuration found in DB");
                    let mmc: MarketMakerConfig = serde_json::from_value(cfg.values.clone()).unwrap();
                    tracing::trace!("    - Configuration: {}: Keccak256: {}", mmc.identifier(), cfg.hash);
                    // Get all instances for this configuration
                    let instances = pull::instances(&db).await.unwrap();
                    tracing::trace!("    - Got {} instances for this configuration", instances.len());

                    // Closing the last instance, if any
                    match instances.last() {
                        Some(instance) => {
                            tracing::info!(
                                "    - Closing last instance (with id: {}) | Initially started at: {}  ⚠️   Make sure to stop the container associated with this instance !",
                                instance.id,
                                instance.started_at
                            );
                            let mut instance: instance::ActiveModel = instance.clone().into();
                            instance.ended_at = Set(Some(chrono::Utc::now().naive_utc()));
                            match instance.update(&db).await {
                                Ok(_) => {}
                                Err(err) => {
                                    tracing::error!("    - Error closing last instance: {}", err);
                                }
                            }
                        }
                        None => {
                            tracing::trace!("    - No instances found for this configuration");
                        }
                    };

                    if let Err(err) = create::instance(&db, msg.config.clone(), msg.commit.clone(), cfg).await {
                        tracing::error!("   - Error attaching instance to configuration: {}", err);
                    }
                }
                None => {
                    tracing::info!("Configuration hash not found in DB. Creating it, and the instance with it ...");
                    match create::configuration(&db, msg.config.clone()).await {
                        Ok(cfg) => {
                            if let Err(err) = create::instance(&db, msg.config.clone(), msg.commit.clone(), &cfg).await {
                                tracing::error!("   - Error attaching instance to configuration: {}", err);
                            }
                        }
                        Err(err) => {
                            tracing::error!("   - Error creating configuration: {}", err);
                        }
                    }
                }
            }
        }
        ParsedMessage::NewPrices(msg) => {
            tracing::trace!("NewPrices received, with reference_price: {} and instance_hash: {}", msg.reference_price, msg.instance_hash);
            // Find the instance by hash
            let db = connect(env.clone()).await.unwrap();

            match pull::instance_by_hash(&db, &msg.instance_hash).await {
                Ok(Some(instance)) => {
                    tracing::debug!("Found instance {} for hash: {}", instance.id, msg.instance_hash);
                    if let Err(err) = create::price(&db, msg, &instance).await {
                        tracing::error!("Error storing price data: {}", err);
                    }
                }
                Ok(None) => {
                    tracing::warn!("Instance not found for hash: {}", msg.instance_hash);
                    // Log available hashes for debugging
                    let instances = pull::instances(&db).await.unwrap();
                    tracing::debug!("Available instance hashes: {:?}", instances.iter().map(|i| &i.hash).collect::<Vec<_>>());
                }
                Err(err) => {
                    tracing::error!("Error finding instance by hash: {}", err);
                }
            }
        }
        ParsedMessage::NewIntent(msg) => {
            tracing::trace!("NewIntent received, with config identifier: {}", msg.config.identifier());
        }
        ParsedMessage::NewTrade(msg) => {
            tracing::trace!("NewTrade received, with config identifier: {}", msg.config.identifier());
        }
        ParsedMessage::Unknown(data) => {
            tracing::warn!("Unknown message type: {:?}", data);
        }
    }
}

pub mod create {

    use crate::{
        entity::{configuration, price, trade},
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
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("🐘 Successfully inserted configuration (id: {})", inserted.id);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("🐘 Error inserting: {}", err);
                Err(err)
            }
        }
    }

    /// Insert a new Bot and return its full Model (with id, timestamps, …)
    pub async fn instance(db: &DatabaseConnection, mmc: MarketMakerConfig, commit: String, cfg: &configuration::Model) -> Result<instance::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let instance_hash = mmc.instance_hash();
        let model = instance::ActiveModel {
            config: Set(config),
            created_at: Set(now),
            updated_at: Set(now),
            configuration_id: Set(Some(cfg.id.clone())),
            started_at: Set(now),
            commit: Set(commit),
            ended_at: Set(None),
            hash: Set(instance_hash),
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("🐘 Successfully inserted instance (id: {}) with hash: {}", inserted.id, inserted.hash);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("🐘 Error inserting: {}", err);
                Err(err)
            }
        }
    }

    /// Insert a new Bot and return its full Model (with id, timestamps, …)
    pub async fn trade(db: &DatabaseConnection, mmc: MarketMakerConfig, instance: &instance::Model) -> Result<trade::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let model = trade::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            instance_id: Set(instance.id.clone()),
            values: Set(json!({})),
            id: Set(Uuid::new_v4().to_string()),
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

    /// Insert a new price record and return its full Model
    pub async fn price(db: &DatabaseConnection, price_data: &crate::types::moni::NewPricesMessage, instance: &instance::Model) -> Result<price::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let price_value = json!({
            "instance_hash": price_data.instance_hash,
            "reference_price": price_data.reference_price,
            "components": price_data.components,
            "timestamp": now.timestamp(),
        });
        let model = price::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            instance_id: Set(instance.id.clone()),
            value: Set(price_value),
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("🐘 Inserted 'price' succeeded: {} for instance: {} (hash: {})", inserted.id, instance.id, price_data.instance_hash);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("🐘 Error inserting price: {}", err);
                Err(err)
            }
        }
    }
}

pub mod pull {
    use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter};

    use crate::entity::{configuration, instance, price, trade};

    pub async fn instances(db: &DatabaseConnection) -> Result<Vec<instance::Model>, sea_orm::DbErr> {
        let models = instance::Entity::find().all(db).await?;
        // tracing::tracing!("Got {} instances in DB", models.len());
        Ok(models)
    }

    pub async fn configurations(db: &DatabaseConnection) -> Result<Vec<configuration::Model>, sea_orm::DbErr> {
        let models = configuration::Entity::find().all(db).await?;
        // tracing::tracing!("Got {} configurations in DB", models.len());
        Ok(models)
    }

    pub async fn trades(db: &DatabaseConnection) -> Result<Vec<trade::Model>, sea_orm::DbErr> {
        let models = trade::Entity::find().all(db).await?;
        // tracing::tracing!("Got {} trades in DB", models.len());
        Ok(models)
    }

    pub async fn prices(db: &DatabaseConnection) -> Result<Vec<price::Model>, sea_orm::DbErr> {
        let models = price::Entity::find().all(db).await?;
        // tracing::tracing!("Got {} prices in DB", models.len());
        Ok(models)
    }

    pub async fn instance_by_hash(db: &DatabaseConnection, hash: &str) -> Result<Option<instance::Model>, sea_orm::DbErr> {
        let instances = instance::Entity::find().all(db).await?;
        Ok(instances.into_iter().find(|inst| inst.hash == hash))
    }
}
