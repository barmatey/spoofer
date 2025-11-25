use crate::connectors::{BinanceConnector, BinanceConnectorConfig, Connector};
use crate::domain::events::Side;
use crate::domain::{display_order_book, OrderBook};
use crate::services::bus::Bus;
use std::time::Duration;

mod connectors;
mod domain;
mod services;

#[tokio::main]
async fn main() {
    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let bus = Bus::new();

    let worker = async || {
        let mut order_book = OrderBook::new();

        loop {
            let events = bus.levels.pull();
            order_book.update(&events);

            display_order_book(&order_book, 10);
            tokio::time::sleep(Duration::from_millis(2_000)).await;
        }
    };

    let mut connector = BinanceConnector::new(
        &bus,
        BinanceConnectorConfig {
            ticker: symbol,
            price_multiply: 100,
            quantity_multiply: 10_000_000,
        },
    );

    tokio::join!(connector.listen(), worker(),);
}
