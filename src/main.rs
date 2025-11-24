use crate::bus::Bus;
use crate::connectors::{BinanceConnector, Connector};
use crate::events::LevelUpdated;

mod events;
mod bus;
mod connectors;
mod temp;


#[tokio::main]
async fn main() {

    let mut bus = Bus::new();
    bus.subscribe::<LevelUpdated>(|ev| {println!("{:?}", ev)});

    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let mut connector = BinanceConnector::new(&bus, &symbol);

    tokio::join!(
        connector.listen(),
        bus.processing()
    );
}