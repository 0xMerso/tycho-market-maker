// main.rs

use futures::executor::block_on;
use sea_orm::{ConnectionTrait, Database, DbBackend, DbErr, Statement};
use shd::types::config::MoniEnvConfig;

// Change this according to your database implementation,
// or supply it as an environment variable.
// the whole database URL string follows the following format:
// "protocol://username:password@host:port/database"
// We put the database name (that last bit) in a separate variable simply for convenience.

async fn run() -> Result<(), DbErr> {
    println!("Running Neon");
    dotenv::from_filename("config/.env.moni.ex").ok(); // Use .env.ex for testing purposes
    let env = MoniEnvConfig::new();
    env.print();

    let db = Database::connect(env.database_url.clone()).await?;
    let db = &match db.get_database_backend() {
        DbBackend::MySql => {
            // db.execute(Statement::from_string(db.get_database_backend(), format!("CREATE DATABASE IF NOT EXISTS `{}`;", env.clone())))
            //     .await?;

            // let url = format!("{}/{}", env.database_url.clone(), env.clone());
            // Database::connect(&url).await?
        }
        DbBackend::Postgres => {
            // db.execute(Statement::from_string(db.get_database_backend(), format!("DROP DATABASE IF EXISTS \"{}\";", env.clone())))
            //     .await?;
            // db.execute(Statement::from_string(db.get_database_backend(), format!("CREATE DATABASE \"{}\";", env.clone())))
            //     .await?;

            // let url = format!("{}/{}", env.database_url.clone(), env.clone());
            // Database::connect(&url).await?
        }
        _ => {
            panic!("Unsupported database backend");
        }
    };
    println!("Neon connected");
    Ok(())
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => println!("Neon finished"),
        Err(err) => println!("Neon error: {}", err),
    }
}
