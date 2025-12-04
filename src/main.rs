use crate::connector::{Connector, ConnectorBuilder, Event};
use crate::level2::{display_books, OrderBook};
use crate::trade::TradeStore;
use futures_util::{stream::select, StreamExt};
use std::pin::pin;
use std::time::{Duration, Instant};
use crate::signal::ArbitrageMonitor;

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

#[tokio::main]
async fn main() {
    let mut builder = ConnectorBuilder::new()
        .add_ticker("btc/usdt", 100, 1_000_000_000)
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info();

    let kraken = builder.build_kraken_connector().unwrap();
    let binance = builder.build_binance_connector().unwrap();

    // 1) создаём стримы
    let kraken_stream = pin!(kraken.stream().await.unwrap());
    let mut binance_stream = pin!(binance.stream().await.unwrap());

    // let mut stream = pin!(select(kraken_stream, binance_stream));

    let mut kraken_book = OrderBook::new("kraken", "btc/usdt", 10);
    let mut binance_book = OrderBook::new("binance", "btc/usdt", 10);

    let mut last_display = Instant::now();

    // 2) читаем события
    while let Some(event) = binance_stream.next().await {
        match event {
            Event::Trade(_) => {}
            Event::LevelUpdate(ev) => {
                kraken_book.update_if_instrument_matches(&ev).unwrap();
                binance_book.update_if_instrument_matches(&ev).unwrap();
            }
        }

        // Обновляем таблицу каждые 200 мс
        if last_display.elapsed() > Duration::from_millis(200) {
            display_books(&[&kraken_book, &binance_book]);
            last_display = Instant::now();
        }
    }
}
