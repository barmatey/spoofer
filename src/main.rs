use crate::connector::{CoinbaseConnector, CoinbaseConnectorConfig, Connector};
use crate::shared::Bus;

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

#[tokio::main]
async fn main() {
    let bus = Bus::new();

    let config = CoinbaseConnectorConfig{
        product_id: "BTC-USD".to_string(),
        price_multiply: 100,
        quantity_multiply: 100_000,
    };


    let printer = tokio::spawn(async move {
        loop {
            let events = bus.trades.pull();
            for ev in events {
                println!("{:?}", ev);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    let listener = tokio::spawn(async move {
        let mut connector = CoinbaseConnector::new(&bus, config);
        connector.listen().await;
    });

    let _ = tokio::join!(printer, listener);

}
