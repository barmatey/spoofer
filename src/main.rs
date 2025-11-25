use std::time::Duration;
use crate::bus::Bus;
use crate::connectors::{BinanceConnector, BinanceConnectorConfig, Connector};
use crate::domain::level2::{display_order_book, OrderBook};
use domain::events::{LevelUpdated};
use crate::domain::events::Side;

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

    let worker = async || {
        let order_book_sub = bus.subscribe::<LevelUpdated>();
        let mut order_book = OrderBook::new();

        loop{
            let events = bus.pull::<LevelUpdated>(order_book_sub).unwrap();
            for ev in events {
                match ev.side {
                    Side::Buy => order_book.update_bid(ev.price, ev.quantity),
                    Side::Sell => order_book.update_ask(ev.price, ev.quantity),
                }
            }
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
