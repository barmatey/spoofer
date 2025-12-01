use std::pin::pin;
use crate::connector::{Connector, ConnectorBuilder};
use futures_util::StreamExt;


mod connector;
mod shared;
mod signal;

mod level2;
mod trade;


#[tokio::main]
async fn main() {
    let mut builder = ConnectorBuilder::new()
        .ticker("BTC/USDT", 100, 100_000_000)
        .subscribe_depth(8)
        .subscribe_trades();

    let binance = builder.build_binance_connector().unwrap();

    // 1) создаём стрим
    let stream = binance.stream().await.unwrap();
    let mut stream = pin!(stream);


    // 2) читаем его
    while let Some(event) = stream.next().await {
        println!("{:?}", event);
    }
}
