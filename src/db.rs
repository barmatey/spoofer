use clickhouse::error::Error;
use clickhouse::Client;
use crate::level2::create_level_updates_table;

pub async fn create_database(client: &Client, db_name: &str) -> Result<(), Error> {
    let query = format!("CREATE DATABASE IF NOT EXISTS {}", db_name);
    client.query(&query).execute().await
}



pub async fn init_database(client: &Client, db_name: &str) -> Result<(), Error> {
    create_database(client, db_name).await?;
    create_level_updates_table(client, db_name).await?;
    Ok(())
}
