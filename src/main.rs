use crate::connector::{BinanceConnector, BinanceConfig, BitstampConnector, BitstampConnectorConfig, Connector, KrakenConnector, KrakenConfig};
use crate::shared::Bus;
use std::sync::Arc;

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

#[tokio::main]
async fn main() {
    let bus = Arc::new(Bus::new());
    let bus2 = bus.clone();
    let bus3 = bus.clone();

    let config = KrakenConfig {
        ticker: "BTC/USDT".to_string(), // не "BTC-USD"
        price_multiply: 1000.0,
        quantity_multiply: 100_000_000.0,
    };

    let printer = tokio::spawn(async move {
        loop {
            let events = bus2.trades.pull();
            for ev in events {
                println!("{:?}", ev);
            }

            let events = bus2.levels.pull();
            for ev in events.iter().take(1) {
                // println!("{:?}", ev);
            }
            tokio::time::sleep(std::time::Duration::from_millis(3_000)).await;
        }
    });

    let listener = tokio::spawn(async move {
        let mut connector = KrakenConnector::new(bus.clone(), config);
        connector.listen().await;
    });

    //
    // let binance_config = BinanceConnectorConfig{
    //     ticker: "btcusdt".to_string(), // не "BTC-USD"
    //     price_multiply: 1000.0,
    //     quantity_multiply: 100_000_000.0,
    // };
    //
    // let binance_listener = tokio::spawn(async move {
    //     let mut connector = BinanceConnector::new(bus3, binance_config);
    //     connector.listen().await;
    // });

    let _ = tokio::join!(printer, listener, );
}
