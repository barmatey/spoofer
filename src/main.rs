mod connector;
mod db;
mod level2;
mod shared;
mod signal;
mod trade;

use crate::connector::{Event, Exchange, StreamConnector};
use db::{ClickHouseClient, SaverService};
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio::task::LocalSet;

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
    let client = ClickHouseClient::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("")
        .with_database("spoofer")
        .build()
        .await
        .unwrap();

    let mut service = SaverService::new(&client, 1_000);

    while let Ok(ev) = rx_events.recv().await {
        service.save(ev).await.unwrap();
    }
    service.flush_all().await.unwrap();
}

async fn processor(mut rx_events: broadcast::Receiver<Event>) {
    while let Ok(ev) = rx_events.recv().await {
        match ev {
            Event::LevelUpdate(v) => {
                // Здесь можно обрабатывать LevelUpdated
            }
            Event::Trade(v) => {
                println!("{:?}", v);
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let (tx_events, _) = broadcast::channel::<Event>(1000);

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
        res = handle_saver => println!("handle_saver: {:?}", res),
        res = handle_processor => println!("handle_processor: {:?}", res),
    }
}
