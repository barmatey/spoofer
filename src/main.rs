use std::pin::pin;
use crate::connector::{Connector, ConnectorBuilder, Event};
use futures_util::StreamExt;


mod connector;
mod shared;
mod signal;

mod level2;
mod trade;


#[tokio::main]
async fn main() {
    let mut builder = ConnectorBuilder::new()
        .ticker("btc/usdt", 100, 100_000_000)
        .subscribe_depth(9)
        .subscribe_trades();

    let kraken = builder.build_kraken_connector().unwrap();

    // 1) создаём стрим
    let stream = kraken.stream().await.unwrap();
    let mut stream = pin!(stream);


    // 2) читаем его
    while let Some(event) = stream.next().await {
        match event {
            Event::Trade(x) => println!("{:?}", x),
            Event::LevelUpdate(_) => {}
        }
    }
}
