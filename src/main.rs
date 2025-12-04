use crate::connector::{Connector, ConnectorBuilder, Event};
use crate::level2::{display_books, OrderBook};
use crate::shared::utils::format_price;
use crate::signal::ArbitrageMonitor;
use crate::trade::TradeStore;
use futures_util::{stream::select, StreamExt};
use std::pin::pin;
use std::time::{Duration, Instant};

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

#[tokio::main]
async fn main() {
    let mut builder = ConnectorBuilder::new()
        .add_ticker("btc/usdt", 100, 1_000_000_000)
        .add_ticker("eth/usdt", 100, 1_000_000_000)

        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info();

    let kraken = builder.build_kraken_connector().unwrap();
    let binance = builder.build_binance_connector().unwrap();

    // 1) создаём стримы
    let kraken_stream = pin!(kraken.stream().await.unwrap());
    let binance_stream = pin!(binance.stream().await.unwrap());

    let mut stream = pin!(select(kraken_stream, binance_stream));

    let mut kraken_book = OrderBook::new("kraken", "btc/usdt", 10);
    let mut binance_book = OrderBook::new("binance", "btc/usdt", 10);

    let mut last_display = Instant::now();

    // 2) читаем события
    while let Some(event) = stream.next().await {
        match event {
            Event::Trade(_) => {}
            Event::LevelUpdate(ev) => {
                kraken_book.update_if_instrument_matches(&ev).unwrap();
                binance_book.update_if_instrument_matches(&ev).unwrap();
            }
        }
        let signal = ArbitrageMonitor::new(&kraken_book, &binance_book, 0.001).execute();
        match signal {
            Some(ev) => println!(
                "Buy: {} on {}. Sell: {} on {}. Profit: {}",
                format_price(ev.buy.price, 2),
                ev.buy.exchange,
                format_price(ev.sell.price, 2),
                ev.sell.exchange,
                ev.profit_pct
            ),
            None => {}
        }
    }
}
