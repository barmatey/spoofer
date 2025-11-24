use crate::bus::Bus;
use crate::connectors::{BinanceConnector, BinanceConnectorConfig, Connector};
use crate::events::{LevelUpdated, TradeEvent};

mod bus;
mod connectors;
mod events;
mod services;

#[tokio::main]
async fn main() {
    let bus = Bus::new();
    bus.subscribe::<LevelUpdated>(|ev| println!("{:?}", ev));
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
