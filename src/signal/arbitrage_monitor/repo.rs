use crate::shared::logger::Logger;
use crate::shared::utils::buffer_service::Callback;

use clickhouse::Client;
use clickhouse::error::Error;
use clickhouse::insert::Insert;
use serde::Serialize;
use crate::signal::arbitrage_monitor::ArbitrageSignal;

#[derive(clickhouse::Row, Serialize)]
struct ArbitrageSignalRow {
    buy_exchange: u8,
    buy_ticker: String,
    buy_price: u64,

    sell_exchange: u8,
    sell_ticker: String,
    sell_price: u64,

    profit_pct: f32,
    profit_abs: Option<f32>,
    timestamp: u64,
}

impl ArbitrageSignalRow {
    pub fn from_signal(sig: &ArbitrageSignal) -> Self {
        Self {
            buy_exchange: sig.buy.exchange.clone() as u8,
            buy_ticker: sig.buy.ticker.to_string(),
            buy_price: sig.buy.price,

            sell_exchange: sig.sell.exchange.clone() as u8,
            sell_ticker: sig.sell.ticker.to_string(),
            sell_price: sig.sell.price,

            profit_pct: sig.profit_pct,
            profit_abs: sig.profit_abs,
            timestamp: sig.timestamp,
        }
    }
}


pub struct ArbitrageSignalRepo<'a> {
    client: &'a Client,
}

impl<'a> ArbitrageSignalRepo<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn save(&self, events: &[ArbitrageSignal]) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        let mut insert: Insert<ArbitrageSignalRow> =
            self.client.insert("arbitrage_signals").await?;

        for ev in events {
            insert.write(&ArbitrageSignalRow::from_signal(ev)).await?;
        }

        insert.end().await?;
        Ok(())
    }
}

impl<'a> Callback<ArbitrageSignal, Error> for ArbitrageSignalRepo<'a> {
    async fn on_buffer_flush(&self, data: &[ArbitrageSignal]) -> Result<(), Error> {
        self.save(data).await
    }
}


pub async fn create_arbitrage_signals_table(
    client: &Client,
    logger: &Logger,
    db_name: &str,
) -> Result<(), Error> {
    logger.info("Creating arbitrage signals table");

    let query = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {}.arbitrage_signals (
            buy_exchange UInt8,
            buy_ticker String,
            buy_price UInt64,

            sell_exchange UInt8,
            sell_ticker String,
            sell_price UInt64,

            profit_pct Float32,
            profit_abs Nullable(Float32),

            timestamp UInt64
        ) ENGINE = MergeTree()
        ORDER BY (timestamp)
    "#,
        db_name
    );

    client.query(&query).execute().await
}
