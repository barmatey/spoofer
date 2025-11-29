use std::sync::Arc;
use crate::connector::{BitstampConnector, BitstampConnectorConfig, Connector};
use crate::shared::Bus;

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

#[tokio::main]
async fn main() {
    let bus = Arc::new(Bus::new());
    let cloned = bus.clone();

    let config = BitstampConnectorConfig{
        ticker: "btcusd".to_string(),   // не "BTC-USD"
        price_multiply: 1000,
        quantity_multiply: 100_000_000,
    };


    let printer = tokio::spawn(async move{
        loop {
            let events = cloned.trades.pull();
            for ev in events {
                println!("{:?}", ev);
            }

            let events = cloned.levels.pull();
            for ev in events{
                // println!("{:?}", ev);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    let listener = tokio::spawn(async move {
        let mut connector = BitstampConnector::new(bus.clone(), config);
        connector.listen().await;
    });

    let _ = tokio::join!(printer, listener);

}
