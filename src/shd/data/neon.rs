// main.rs

use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, DbErr, EntityTrait, Set};
use serde_json::json;

use crate::{
    entity::instance,
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
    tracing::info!("Connecting to database: {}", env.database_url);
    match Database::connect(env.database_url.clone()).await {
        Ok(db) => {
            tracing::info!("Successfully connected to database");
            Ok(db)
        }
        Err(err) => {
            tracing::error!("Failed to connect to database: {}", err);
            Err(err)
        }
    }
}

/// Handle different message types (from Redis pub-sub, to then push to DB)
pub async fn handle(msg: &ParsedMessage, env: MoniEnvConfig) {
    // Connect to database once for this message
    let db = match connect(env.clone()).await {
        Ok(db) => db,
        Err(err) => {
            tracing::error!("Failed to connect to database for message handling: {}", err.to_string());
            return;
        }
    };

    match msg {
        ParsedMessage::Ping => {
            tracing::trace!("Ping received !");
        }
        ParsedMessage::NewInstance(msg) => {
            tracing::trace!("NewInstance received with config identifier: {}", msg.config.id());
            let config_hash = msg.config.hash();
            tracing::trace!("Config Keccak256: {}", config_hash);

            let cfgs = match pull::configurations(&db).await {
                Ok(cfgs) => cfgs,
                Err(err) => {
                    tracing::error!("Failed to pull configurations: {}", err);
                    return;
                }
            };

            let hash = config_hash.to_lowercase();

            if let Some(cfg) = cfgs.iter().find(|cfg| cfg.hash.to_lowercase() == hash) {
                tracing::info!("Configuration found in DB");
                let mmc: MarketMakerConfig = match serde_json::from_value(cfg.values.clone()) {
                    Ok(mmc) => mmc,
                    Err(err) => {
                        tracing::error!("Failed to deserialize configuration: {}", err);
                        return;
                    }
                };
                tracing::trace!("    - Configuration: {}: Keccak256: {}", mmc.id(), cfg.hash);

                let instances = match pull::instances(&db).await {
                    Ok(instances) => instances,
                    Err(err) => {
                        tracing::error!("Failed to pull instances: {}", err);
                        return;
                    }
                };

                tracing::trace!("    - Got {} instances for this configuration", instances.len());

                // Closing the last instance, if any
                if let Some(instance) = instances.last() {
                    tracing::info!(
                        "    - Closing last instance (with id: {}) | Initially started at: {}  ⚠️   Make sure to stop the container associated with this instance !",
                        instance.id,
                        instance.started_at
                    );
                    let mut instance: instance::ActiveModel = instance.clone().into();
                    instance.ended_at = Set(Some(chrono::Utc::now().naive_utc()));

                    if let Err(err) = instance.update(&db).await {
                        tracing::error!("    - Error closing last instance: {}", err);
                    }
                } else {
                    tracing::trace!("    - No instances found for this configuration");
                }

                if let Err(err) = create::instance(&db, cfg, msg.config.clone(), msg.identifier.clone(), msg.commit.clone()).await {
                    tracing::error!("   - Error attaching instance to configuration: {}", err);
                }
            } else {
                tracing::info!("Configuration hash not found in DB. Creating it, and the instance with it ...");

                match create::configuration(&db, msg.config.clone()).await {
                    Ok(cfg) => {
                        if let Err(err) = create::instance(&db, &cfg, msg.config.clone(), msg.identifier.clone(), msg.commit.clone()).await {
                            tracing::error!("   - Error attaching instance to configuration: {}", err);
                        }
                    }
                    Err(err) => {
                        tracing::error!("   - Error creating configuration: {}", err);
                    }
                }
            }
        }
        ParsedMessage::NewPrices(msg) => {
            tracing::trace!("NewPrices received, with reference_price: {} and instance identifier: {}", msg.reference_price, msg.identifier);

            let instances = match pull::instances(&db).await {
                Ok(instances) => instances,
                Err(err) => {
                    tracing::error!("Error finding instance by hash: {}", err);
                    return;
                }
            };

            if let Some(instance) = instances.into_iter().find(|inst| inst.identifier == msg.identifier) {
                if let Err(err) = create::price(&db, &instance, msg).await {
                    tracing::error!("Error storing price data: {}", err);
                }
            } else {
                tracing::warn!("Instance not found for hash: {}", msg.identifier);
            }
        }
        ParsedMessage::NewTrade(msg) => {
            tracing::trace!("NewTrade received, with instance identifier: {}", msg.identifier);

            let instances = match pull::instances(&db).await {
                Ok(instances) => instances,
                Err(err) => {
                    tracing::error!("Error finding instance by hash: {}", err);
                    return;
                }
            };

            if let Some(instance) = instances.into_iter().find(|inst| inst.identifier == msg.identifier) {
                if let Err(err) = create::trade(&db, &instance, msg).await {
                    tracing::error!("Error storing trade data: {}", err);
                }
                tracing::info!("Trade data stored successfully");
            } else {
                tracing::warn!("Instance not found for hash: {}", msg.identifier);
            }
        }
        ParsedMessage::Unknown(data) => {
            tracing::warn!("Unknown message type: {:?}", data);
        }
    }
}

pub mod create {
    use crate::types::{
        config::MarketMakerConfig,
        moni::{NewPricesMessage, NewTradeMessage},
    };

    use crate::entity::{configuration, instance, price, trade};

    use super::*;

    pub async fn configuration(db: &DatabaseConnection, mmc: MarketMakerConfig) -> Result<configuration::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let model = configuration::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            values: Set(config),
            hash: Set(mmc.hash()),
            chain_id: Set(mmc.chain_id as i32),
            base_token_address: Set(mmc.base_token_address),
            quote_token_address: Set(mmc.quote_token_address),
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => {
                tracing::info!("Successfully inserted configuration: {}", inserted.id);
                Ok(inserted)
            }
            Err(err) => {
                tracing::error!("Error inserting: {}", err);
                Err(err)
            }
        }
    }

    /// Insert a new Bot and return its full Model (with id, timestamps, …)
    pub async fn instance(db: &DatabaseConnection, cfg: &configuration::Model, mmc: MarketMakerConfig, identifier: String, commit: String) -> Result<instance::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let config = json!(mmc);
        let model = instance::ActiveModel {
            config: Set(config),
            created_at: Set(now),
            updated_at: Set(now),
            configuration_id: Set(Some(cfg.id.clone())),
            started_at: Set(now),
            commit: Set(commit),
            ended_at: Set(None),
            identifier: Set(identifier.clone()),
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => Ok(inserted),
            Err(err) => {
                tracing::error!("Error inserting: {}", err);
                Err(err)
            }
        }
    }

    /// Insert a new price record and return its full Model
    pub async fn price(db: &DatabaseConnection, instance: &instance::Model, msg: &NewPricesMessage) -> Result<price::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let model = price::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            instance_id: Set(instance.id.clone()),
            value: Set(json!(msg)),
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => Ok(inserted),
            Err(err) => {
                tracing::error!("Error inserting: {}", err);
                Err(err)
            }
        }
    }

    /// Insert a new trade record and return its full Model
    pub async fn trade(db: &DatabaseConnection, instance: &instance::Model, msg: &NewTradeMessage) -> Result<trade::Model, sea_orm::DbErr> {
        let now = chrono::Utc::now().naive_utc();
        let model = trade::ActiveModel {
            created_at: Set(now),
            updated_at: Set(now),
            instance_id: Set(instance.id.clone()),
            values: Set(json!(msg)),
            id: Set(Uuid::new_v4().to_string()),
        };
        match model.insert(db).await {
            Ok(inserted) => Ok(inserted),
            Err(err) => {
                tracing::error!("Error inserting: {}", err);
                Err(err)
            }
        }
    }
}

pub mod pull {

    use crate::entity::{configuration, instance, price, trade};

    use super::*;

    pub async fn instances(db: &DatabaseConnection) -> Result<Vec<instance::Model>, sea_orm::DbErr> {
        instance::Entity::find().all(db).await
    }

    pub async fn configurations(db: &DatabaseConnection) -> Result<Vec<configuration::Model>, sea_orm::DbErr> {
        crate::entity::configuration::Entity::find().all(db).await
    }

    pub async fn trades(db: &DatabaseConnection) -> Result<Vec<trade::Model>, sea_orm::DbErr> {
        trade::Entity::find().all(db).await
    }

    pub async fn prices(db: &DatabaseConnection) -> Result<Vec<price::Model>, sea_orm::DbErr> {
        price::Entity::find().all(db).await
    }
}
