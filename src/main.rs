use crate::connector::{Connector, ConnectorBuilder, Event};
use futures_util::StreamExt;
use std::pin::pin;

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

#[tokio::main]
async fn main() {
    let mut builder = ConnectorBuilder::new()
        .add_ticker("btc/usdt", 100, 100_000_000)
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info();

    let kraken = builder.build_kraken_connector().unwrap();
    let binance =  builder.build_binance_connector().unwrap();

    // 1) создаём стрим
    let stream = kraken.stream().await.unwrap();
    let mut stream = pin!(stream);

    // 2) читаем его
    while let Some(event) = stream.next().await {
        match event {
            Event::Trade(x) => {
                // println!("{:?}", x)
            },
            Event::LevelUpdate(y) => {
                // println!("{:?}", y)
            }
        }
    }
}
