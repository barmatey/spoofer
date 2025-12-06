mod connector;
mod db;
mod level2;
mod shared;
mod signal;
mod trade;

use crate::connector::{Event, Exchange, StreamConnector};
use db::{ClickHouseClient, SaverService};
use futures_util::StreamExt;
use tokio::sync::{mpsc};

static TICKERS: [(&'static str, u32, u32); 4] = [
    ("btc/usdt", 100, 1_000_000),
    ("eth/usdt", 100, 10_000),
    ("sol/usdt", 1000, 10_000),
    ("bnb/usdt", 1000, 10_000),
];

async fn stream(tx_saver: mpsc::Sender<Event>, tx_processor: mpsc::Sender<Event>) {
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
        let _ = tx_saver.send(event.clone()).await;
        let _ = tx_processor.send(event).await;
    }
}

async fn saver(mut rx_events: mpsc::Receiver<Event>) {
    let client = ClickHouseClient::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("")
        .with_database("spoofer")
        .build()
        .await
        .unwrap();

    let mut service = SaverService::new(&client, 10_000);

    loop {
        match rx_events.recv().await {
            Some(ev) => {
                service.save(ev).await.unwrap();
            }
            None => {
                eprintln!("Saver: broadcast channel closed");
                panic!();
            }
        }
    }
}

async fn processor(mut rx_events: mpsc::Receiver<Event>) {
    loop {
        match rx_events.recv().await {
            Some(ev) => {
                println!("{:?}", ev);
            }
            None => {
                eprintln!("Saver: broadcast channel closed");
                panic!();
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let (tx_saver, rx_saver) = mpsc::channel::<Event>(100);
    let (tx_processor, rx_processor) = mpsc::channel::<Event>(100);

    // Stream tread
    let handle_stream = tokio::spawn(async move {
        stream(tx_saver, tx_processor).await;
    });

    // Saver thread
    let handle_saver = tokio::spawn(async move {
        saver(rx_saver).await;
    });

    // Arbitrage tread
    let handle_processor = tokio::spawn(async move {
        processor(rx_processor).await;
    });

    // Ждем все задачи (они бесконечные)
    tokio::select! {
        res = handle_stream => println!("handle_stream: {:?}", res),
        res = handle_saver => println!("handle_saver: {:?}", res),
        res = handle_processor => println!("handle_processor: {:?}", res),
    }
}
