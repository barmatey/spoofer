mod connector;
mod db;
mod level2;
mod shared;
mod signal;
mod trade;

use crate::connector::{Event, StreamConnector};
use crate::db::init_database;
use crate::level2::LevelUpdatedRepo;
use clickhouse::Client;
use futures_util::StreamExt;
use tokio::sync::broadcast;

static TICKERS: [(&'static str, u32, u32); 4] = [
    ("btc/usdt", 100, 1_000_000),
    ("eth/usdt", 100, 10_000),
    ("sol/usdt", 1000, 10_000),
    ("bnb/usdt", 1000, 10_000),
];

async fn stream(tx_events: broadcast::Sender<Event>) {
    let mut connector = StreamConnector::new()
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info();

    for (ticker, p_mult, q_mult) in TICKERS {
        connector = connector.add_ticker(ticker, p_mult, q_mult);
    }

    let mut stream = connector.connect().await.unwrap();
    while let Some(event) = stream.next().await {
        let _ = tx_events.send(event);
    }
}

async fn saver(mut rx_events: broadcast::Receiver<Event>) {
    let client = Client::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("");
    init_database(&client, "spoofer", true).await.unwrap();
    let client = client.with_database("spoofer");

    let buffer_size = 1_000;
    let mut levels = Vec::with_capacity(buffer_size);
    let mut trades = Vec::with_capacity(buffer_size);

    while let Ok(ev) = rx_events.recv().await {
        match ev {
            Event::LevelUpdate(v) => {
                levels.push(v);
                if levels.len() >= buffer_size {
                    LevelUpdatedRepo::new(&client).save(&levels).await.unwrap();
                    levels.clear();
                }
            }
            Event::Trade(v) => {
                trades.push(v);
            }
        }
    }
}

async fn processor(mut rx_events: broadcast::Receiver<Event>) {
    while let Ok(ev) = rx_events.recv().await {
        match ev {
            Event::LevelUpdate(v) => {
                // Здесь можно обрабатывать LevelUpdated
                println!("Processor got LevelUpdate: {:?}", v);
            }
            Event::Trade(_) => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let (tx_events, _) = broadcast::channel::<Event>(1000);

    let handle_saver = tokio::spawn(saver(tx_events.subscribe()));
    let handle_processor = tokio::spawn(processor(tx_events.subscribe()));
    let handle_stream = tokio::task::spawn_local(stream(tx_events));

    tokio::select! {
        res = handle_stream => {
            if let Err(e) = res {
                eprintln!("Stream task error: {:?}", e);
                std::process::exit(1);
            }
        }
        res = handle_saver => {
            if let Err(e) = res {
                eprintln!("Saver task error: {:?}", e);
                std::process::exit(1);
            }
        }
        res = handle_processor => {
            if let Err(e) = res {
                eprintln!("Processor task error: {:?}", e);
                std::process::exit(1);
            }
        }
    }

    std::process::exit(0);
}
