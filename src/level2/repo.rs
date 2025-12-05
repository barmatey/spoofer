use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::logger::Logger;
use clickhouse::error::Error;
use clickhouse::insert::Insert;
use clickhouse::Client;
use serde::Serialize;

#[derive(clickhouse::Row, Serialize)]
struct LevelUpdateRow {
    exchange: String,
    ticker: String,
    side: u8,
    price: u64,
    quantity: u64,
    timestamp: u64,
}

impl LevelUpdateRow {
    pub fn from_level_updated(ev: &LevelUpdated) -> Self {
        Self {
            exchange: ev.exchange.to_string(),
            ticker: ev.ticker.to_string(),
            side: ev.side as u8,
            price: ev.price,
            quantity: ev.quantity,
            timestamp: ev.timestamp,
        }
    }
}

pub struct LevelUpdatedRepo<'a> {
    client: &'a Client,
}

impl<'a> LevelUpdatedRepo<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn save(&self, events: &[LevelUpdated]) -> Result<(), Level2Error> {
        if events.is_empty() {
            return Ok(());
        }

        let mut insert: Insert<LevelUpdateRow> = self.client.insert("level_updates").await?;
        for ev in events {
            insert
                .write(&LevelUpdateRow::from_level_updated(ev))
                .await?;
        }
        insert.end().await?;
        Ok(())
    }
}

pub async fn create_level_updates_table(
    client: &Client,
    logger: &Logger,
    db_name: &str,
) -> Result<(), Error> {
    logger.info("Create level updated table");

    let query = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {}.level_updates (
            exchange String,
            ticker String,
            side UInt8,
            price UInt64,
            quantity UInt64,
            timestamp UInt64
        ) ENGINE = MergeTree()
        ORDER BY (exchange, ticker, timestamp)
    "#,
        db_name
    );
    client.query(&query).execute().await
}
