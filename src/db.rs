use clickhouse::Client;
use clickhouse::error::Error;

pub async fn create_database(client: &Client, db_name: &str) -> Result<(), Error> {
    let query = format!("CREATE DATABASE IF NOT EXISTS {}", db_name);
    client.query(&query).execute().await
}

pub async fn create_level_updates_table(client: &Client, db_name: &str) -> Result<(), Error> {
    let query = format!(r#"
        CREATE TABLE IF NOT EXISTS {}.level_updates (
            exchange String,
            ticker String,
            side UInt8,
            price UInt64,
            quantity UInt64,
            timestamp UInt64
        ) ENGINE = MergeTree()
        ORDER BY (exchange, ticker, timestamp)
    "#, db_name);

    client.query(&query).execute().await
}
