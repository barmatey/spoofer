use crate::connector::{Connector, ConnectorBuilder};
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
        let mut connector = ConnectorBuilder::new(bus3)
            .tickers(&["BTC/USDT", "ETH/USDT"])
            .subscribe_trades()
            .build_binance_connector()
            .unwrap();
        connector.listen().await;
    });

    let _ = tokio::join!(printer, listener,);
}
