use crate::connector::{Connector, ConnectorBuilder, Event};
use crate::level2::OrderBook;
use crate::trade::TradeStore;
use futures_util::{stream::select, StreamExt};
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
    let binance = builder.build_binance_connector().unwrap();

    // 1) создаём стрим
    let kraken_stream = pin!(kraken.stream().await.unwrap());
    let binance_stream = pin!(binance.stream().await.unwrap());

    let mut stream = pin!(select(kraken_stream, binance_stream));

    let mut book = OrderBook::new("kraken","btc/usdt");
    let mut trades = TradeStore::new("kraken","btc/usdt", 100);

    // 2) читаем его
    while let Some(event) = stream.next().await {
        match event {
            Event::Trade(x) => {
                trades.update_if_instrument_matches(x).unwrap();
                println!("{}", trades.trades().len())
            }
            Event::LevelUpdate(y) => {
                book.update_if_instrument_matches(y).unwrap();
            }
        }
    }
}
