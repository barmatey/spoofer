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
    let mut builder = ConnectorBuilder::new().subscribe_depth(10).log_level_info();
    let tickers = [
        ("btc/usdt", 100, 1_000_000),
        ("eth/usdt", 100, 10_000),
        ("sol/usdt", 1000, 10_000),
        ("bnb/usdt", 1000, 10_000),
    ];

    for (ticker, p_mult, q_mult) in tickers {
        builder = builder.add_ticker(ticker, p_mult, q_mult);
    }

    let mut books = vec![];
    for (ticker, _, _) in tickers {
        let kraken_book = OrderBook::new("kraken", ticker, 10);
        let binance_book = OrderBook::new("binance", ticker, 10);
        books.push((kraken_book, binance_book));
    }

    // 1) создаём стримы
    let kraken = builder.build_kraken_connector().unwrap();
    let binance = builder.build_binance_connector().unwrap();
    let kraken_stream = pin!(kraken.stream().await.unwrap());
    let binance_stream = pin!(binance.stream().await.unwrap());
    let mut stream = pin!(select(kraken_stream, binance_stream));

    // 2) читаем события
    while let Some(event) = stream.next().await {
        match event {
            Event::Trade(_) => {}
            Event::LevelUpdate(ev) => {
                for (kraken_book, binance_book) in books.iter_mut() {
                    kraken_book.update_if_instrument_matches(&ev).unwrap();
                    binance_book.update_if_instrument_matches(&ev).unwrap();

                    let signal =
                        ArbitrageMonitor::new(&kraken_book, &binance_book, 0.002).execute();
                    match signal {
                        Some(ev) => println!(
                            "[{}] Buy: {} on {}. Sell: {} on {}. Profit: {}. Timestamp: {}",
                            ev.buy.ticker,
                            format_price(ev.buy.price, 2),
                            ev.buy.exchange,
                            format_price(ev.sell.price, 2),
                            ev.sell.exchange,
                            ev.profit_pct,
                            ev.timestamp,
                        ),
                        None => {}
                    }
                }
            }
        }
    }
}
