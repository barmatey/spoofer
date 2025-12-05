use crate::level2::create_level_updates_table;
use crate::shared::logger::Logger;
use clickhouse::error::Error;
use clickhouse::Client;
use tracing::Level;

pub async fn create_database(client: &Client, logger: &Logger, db_name: &str) -> Result<(), Error> {
    let query_check = format!(
        "SELECT name FROM system.databases WHERE name = '{}'",
        db_name
    );

    let existing: Vec<String> = client.query(&query_check).fetch_all().await?;

    if existing.is_empty() {
        logger.info(&format!("Database '{}' does not exist. Creating...", db_name));
        let query_create = format!("CREATE DATABASE {}", db_name);
        client.query(&query_create).execute().await?;
        logger.info(&format!("Database '{}' created", db_name));
    } else {
        logger.info(&format!("Database '{}' already exists", db_name));
    }

    Ok(())
}



pub async fn drop_all_tables(client: &Client, logger: &Logger, db_name: &str) -> Result<(), Error> {
    logger.info(&format!("Drop all tables in database {}", db_name));

    let tables: Vec<String> = client
        .query(&format!("SHOW TABLES FROM {}", db_name))
        .fetch_all()
        .await?;

    for table in tables {
        let drop_query = format!("DROP TABLE IF EXISTS {}.{}", db_name, table);
        logger.info(&format!("Dropping table {}", table));
        client.query(&drop_query).execute().await?;
    }

    Ok(())
}



pub async fn init_database(client: &Client, db_name: &str, recreate: bool) -> Result<(), Error> {
    let logger = Logger::new("initialisation", Level::INFO);
    create_database(client, &logger, db_name).await?;
    if recreate{
        drop_all_tables(client, &logger, db_name).await?;
    }
    create_level_updates_table(client, &logger, db_name).await?;
    Ok(())
}
