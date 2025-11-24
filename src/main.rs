use crate::bus::Bus;
use crate::connectors::{BinanceConnector, Connector};
use crate::events::{LevelUpdated, TradeEvent};

mod events;
mod bus;
mod connectors;
mod temp;
mod services;

#[tokio::main]
async fn main() {

    let bus = Bus::new();
    bus.subscribe::<LevelUpdated>(|ev| {println!("{:?}", ev)});
    bus.subscribe::<TradeEvent>(|ev| {println!("{:?}", ev)});

    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let mut connector = BinanceConnector::new(&bus, &symbol);

    tokio::join!(
        connector.listen(),
        bus.processing()
    );
}