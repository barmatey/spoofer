use crate::bus::Bus;
use crate::connectors::{BinanceConnector, BinanceConnectorConfig, Connector};
use domain::events::{LevelUpdated, TradeEvent};
use crate::domain::events::Side;
use crate::domain::level2::OrderBook;

mod bus;
mod connectors;
mod services;
mod domain;

#[tokio::main]
async fn main() {
    let bus = Bus::new();
    let mut order_book = OrderBook::new();

    bus.subscribe::<LevelUpdated>(|ev| {
        match ev.side {
            Side::Buy =>order_book.update_bid(ev.price, ev.quantity),
            Side::Sell => order_book.update_ask(ev.price, ev.quantity),
        }
    });
    bus.subscribe::<TradeEvent>(|ev| println!("{:?}", ev));

    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let mut connector = BinanceConnector::new(
        &bus,
        BinanceConnectorConfig {
            ticker: symbol,
            price_multiply: 100,
            quantity_multiply: 10_000_000,
        },
    );

    tokio::join!(connector.listen(), bus.processing());
}
