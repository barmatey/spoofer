use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::logger::Logger;
use clickhouse::error::Error;
use clickhouse::insert::Insert;
use clickhouse::Client;
use serde::Serialize;
use std::sync::Arc;

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
    buffer_size: usize,
    buffer: Vec<LevelUpdated>,
}

impl<'a> LevelUpdatedRepo<'a> {
    pub fn new(client: &'a Client, buffer_size: usize) -> Self {
        Self {
            client,
            buffer_size,
            buffer: Vec::with_capacity(buffer_size + 1),
        }
    }

    pub fn push(&mut self, event: LevelUpdated) {
        self.buffer.push(event);
    }

    pub async fn save_if_full(&mut self) -> Result<(), Level2Error> {
        if self.buffer.len() >= self.buffer_size {
            self.save().await?
        }
        Ok(())
    }

    pub async fn save(&mut self) -> Result<(), Level2Error> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let mut insert: Insert<LevelUpdateRow> = self.client.insert("level_updates").await?;

        for event in self.buffer.iter() {
            let row = LevelUpdateRow::from_level_updated(event);
            insert.write(&row).await?;
        }

        insert.end().await?;
        self.buffer.clear();
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
