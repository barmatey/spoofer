mod connector;
mod db;
mod level2;
mod shared;
mod signal;
mod trade;

use crate::connector::{Event, Exchange, StreamConnector};
use crate::level2::OrderBook;
use crate::signal::ArbitrageMonitor;
use db::{DatabaseClient, SaverService};
use futures_util::StreamExt;
use tokio::sync::broadcast;

static TICKERS: [(&'static str, u32, u32); 4] = [
    ("btc/usdt", 100, 1_000_000),
    ("eth/usdt", 100, 10_000),
    ("sol/usdt", 1000, 10_000),
    ("bnb/usdt", 1000, 10_000),
];

async fn stream(tx_events: broadcast::Sender<Event>) {
    let mut stream = StreamConnector::new()
        .exchanges(&[Exchange::All])
        .tickers(&TICKERS)
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info()
        .connect()
        .await
        .unwrap();
    while let Some(event) = stream.next().await {
        let _ = tx_events.send(event);
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
    let mut service = SaverService::new(&client, 50_000);
    loop {
        let event = rx_events.recv().await.unwrap();
        service.save(event).await.unwrap();
    }
}

async fn processor(mut rx_events: broadcast::Receiver<Event>) {
    let mut books = vec![];
    for (ticker, _, _) in TICKERS.iter() {
        let ob1 = OrderBook::new("kraken", ticker, 10);
        let ob2 = OrderBook::new("binance", ticker, 10);
        books.push((ob1, ob2));
    }
    loop {
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(_v) => {}
            Event::LevelUpdate(v) => {
                for pair in books.iter_mut() {
                    pair.0.update_or_miss(&v);
                    pair.1.update_or_miss(&v);
                    let signal = ArbitrageMonitor::new(&pair.0, &pair.1, 0.0002).execute();
                    signal.map(|x| println!("{:?}", x));
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

    // Ждем все задачи (они бесконечные)
    tokio::select! {
        res = handle_stream => println!("handle_stream: {:?}", res),
        // res = handle_saver => println!("handle_saver: {:?}", res),
        res = handle_processor => println!("handle_processor: {:?}", res),
    }
}
