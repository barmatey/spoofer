use crate::connectors::{BinanceConnector, Connector};

mod events;
mod bus;
mod connectors;
mod temp;


#[tokio::main]
async fn main() {

    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let mut connector = BinanceConnector::new(&symbol);
    connector.listen().await;
}