mod connector;
mod db;
mod level2;
mod shared;
mod signal;
mod trade;

use crate::connector::{Event, StreamConnector};
use crate::level2::{LevelUpdatedRepo, OrderBook};
use crate::shared::utils::buffer_service::BufferService;
use crate::shared::Exchange;
use crate::signal::arbitrage_monitor::{ArbitrageMonitor, ArbitrageSignalRepo};
use crate::trade::TradeEventRepo;
use db::DatabaseClient;
use futures_util::StreamExt;
use tokio::sync::broadcast;

// Ticker, multiply for price, multiply for quantity
static TICKERS: [(&'static str, u32, u32); 4] = [
    ("btc/usdt", 100, 1_000_000),
    ("eth/usdt", 100, 10_000),
    ("sol/usdt", 1000, 10_000),
    ("bnb/usdt", 1000, 10_000),
];

async fn stream(tx_events: broadcast::Sender<Event>) {
    let mut stream = StreamConnector::new()
        .exchanges(&[Exchange::Binance, Exchange::Kraken])
        .tickers(&TICKERS)
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info()
        .connect()
        .await
        .unwrap();
    loop {
        let event = stream.next().await.unwrap();
        tx_events.send(event).unwrap();
    }
}

async fn saver(mut rx_events: broadcast::Receiver<Event>) {
    let client = DatabaseClient::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("")
        .with_database("spoofer")
        .build()
        .await
        .unwrap();
    let trade_saver = BufferService::new(TradeEventRepo::new(&client), 10_000);
    let level2saver = BufferService::new(LevelUpdatedRepo::new(&client), 50_000);
    loop {
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(v) => trade_saver.push(v).await.unwrap(),
            Event::LevelUpdate(v) => level2saver.push(v).await.unwrap(),
        };
    }
}

async fn processor(mut rx_events: broadcast::Receiver<Event>) {
    let client = DatabaseClient::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("")
        .with_database("spoofer")
        .build()
        .await
        .unwrap();

    let mut books = vec![];
    for (ticker, _, _) in TICKERS.iter() {
        let ob1 = OrderBook::new(Exchange::Binance, ticker, 10);
        let ob2 = OrderBook::new(Exchange::Kraken, ticker, 10);
        books.push((ob1, ob2));
    }
    loop {
        let signal_saver = BufferService::new(ArbitrageSignalRepo::new(&client), 100);
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(_v) => {}
            Event::LevelUpdate(v) => {
                for pair in books.iter_mut() {
                    pair.0.update_or_miss(&v);
                    pair.1.update_or_miss(&v);
                    let signal = ArbitrageMonitor::new(&pair.0, &pair.1, 0.0003).execute();
                    signal.map(async |x| {
                        println!("{:?}", x);
                        signal_saver.push(x).await.unwrap();
                    });
                }
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let (tx_events, _) = broadcast::channel::<Event>(50_000);

    // Stream tread
    let stream_tx = tx_events.clone();
    let handle_stream = tokio::spawn(async move {
        stream(stream_tx).await;
    });

    // Saver thread
    let saver_rx = tx_events.subscribe();
    let handle_saver = tokio::spawn(async move {
        saver(saver_rx).await;
    });

    // Arbitrage tread
    let processor_rx = tx_events.subscribe();
    let handle_processor = tokio::spawn(async move {
        processor(processor_rx).await;
    });

    tokio::select! {
        res = handle_stream => println!("handle_stream: {:?}", res),
        res = handle_saver => println!("handle_saver: {:?}", res),
        res = handle_processor => println!("handle_processor: {:?}", res),
    }
}
