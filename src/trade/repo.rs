use clickhouse::{insert::Insert, Client, Row};
use serde::Serialize;

use crate::shared::logger::Logger;
use crate::shared::{Price, Quantity, TimestampMS};
use crate::trade::{TradeError, TradeEvent};

#[derive(Row, Serialize)]
pub struct TradeEventRow {
    exchange: String,
    ticker: String,
    price: Price,
    quantity: Quantity,
    timestamp: TimestampMS,
    market_maker: u8,
}

impl TradeEventRow {
    pub fn from_trade(ev: &TradeEvent) -> Self {
        Self {
            exchange: ev.exchange.as_ref().clone(),
            ticker: ev.ticker.as_ref().clone(),
            price: ev.price,
            quantity: ev.quantity,
            timestamp: ev.timestamp,
            market_maker: ev.market_maker as u8,
        }
    }
}

pub struct TradeEventRepo<'a> {
    client: &'a Client,
}

impl<'a> TradeEventRepo<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn save(&self, events: &[TradeEvent]) -> Result<(), TradeError> {
        if events.is_empty() {
            return Ok(());
        }

        let mut insert: Insert<TradeEventRow> = self.client.insert("trade_events").await?;

        for ev in events {
            insert.write(&TradeEventRow::from_trade(ev)).await?;
        }

        insert.end().await?;
        Ok(())
    }
}

pub async fn create_trade_event_table(
    client: &Client,
    logger: &Logger,
    db_name: &str,
) -> Result<(), TradeError> {
    logger.info("Creating trade events table");

    let query = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {}.trade_events (
            exchange String,
            ticker String,
            price UInt64,
            quantity UInt64,
            timestamp UInt64,
            market_maker UInt8
        ) ENGINE = MergeTree()
        ORDER BY (exchange, ticker, timestamp)
    "#,
        db_name
    );

    client.query(&query).execute().await?;
    Ok(())
}
