use std::time::Duration;
use crate::bus::Bus;
use crate::connectors::{BinanceConnector, BinanceConnectorConfig, Connector};
use crate::domain::level2::OrderBook;
use domain::events::{LevelUpdated};

mod bus;
mod connectors;
mod domain;
mod services;

#[tokio::main]
async fn main() {
    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let bus = Bus::new();

    let order_book_sub = bus.subscribe::<LevelUpdated>();

    let mut order_book = OrderBook::new();
    let worker = async || {
        loop{
            let events = bus.pull::<LevelUpdated>(order_book_sub).unwrap();
            for ev in events {
                println!("{:?}", ev);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
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
