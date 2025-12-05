use crate::connector::{Connector, ConnectorBuilder, Event};
use crate::db::init_database;
use crate::level2::{LevelUpdated, LevelUpdatedRepo, OrderBook};
use crate::shared::utils::format_price;
use crate::signal::ArbitrageMonitor;
use clickhouse::Client;
use futures_util::StreamExt;
use pin_utils::pin_mut;
use tokio::sync::mpsc;
use tokio::task::LocalSet;
// добавь в Cargo.toml: pin-utils = "0.1"

mod connector;
mod shared;
mod signal;

mod level2;
mod trade;

mod db;

async fn stream(tx_events: mpsc::Sender<Event>) {
    // Настройка коннекторов
    let mut builder = ConnectorBuilder::new().subscribe_depth(10).log_level_info();

    // Тикеры
    let tickers = [
        ("btc/usdt", 100, 1_000_000),
        ("eth/usdt", 100, 10_000),
        ("sol/usdt", 1000, 10_000),
        ("bnb/usdt", 1000, 10_000),
    ];

    for (ticker, p_mult, q_mult) in tickers {
        builder = builder.add_ticker(ticker, p_mult, q_mult);
    }

    // Connectors
    let kraken = builder.build_kraken_connector().unwrap();
    let binance = builder.build_binance_connector().unwrap();

    // Stream
    let kraken_stream = kraken.stream().await.unwrap();
    let binance_stream = binance.stream().await.unwrap();

    pin_mut!(kraken_stream);
    pin_mut!(binance_stream);

    // Чтение из обоих стримов и отправка в канал
    loop {
        tokio::select! {
            Some(event) = kraken_stream.next() => {
                let _ = tx_events.send(event).await;
            }
            Some(event) = binance_stream.next() => {
                let _ = tx_events.send(event).await;
            }
            else => break, // оба потока закрыты
        }
    }
}

async fn saver(rx_events: mpsc::Receiver<Event>) {
    let client = Client::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("");

    init_database(&client, "spoofer", true).await.unwrap();
    let client = client.with_database("spoofer");

    let mut depth_repo = LevelUpdatedRepo::new(&client, 1_000);

    let mut rx_events = rx_events;
    while let Some(ev) = rx_events.recv().await {
        match ev {
            Event::LevelUpdate(v) => {
                depth_repo.push(v);
                depth_repo.save_if_full().await.unwrap();
            }
            Event::Trade(v) => {}
        }
    }
}

fn calculating_task(rx_events: mpsc::Receiver<LevelUpdated>) {
    todo!()
}

#[tokio::main]
async fn main() {
    let (tx_events, rx_events) = mpsc::channel::<Event>(1000);

    let local = LocalSet::new();
    local
        .run_until(async move {
            let handle_stream = tokio::task::spawn_local(stream(tx_events));
            let handle_saver = tokio::spawn(saver(rx_events));

            tokio::select! {
                res = handle_stream => res.unwrap_or_else(|e| {
                    eprintln!("Stream task error: {:?}", e);
                    std::process::exit(1);
                }),
                res = handle_saver => res.unwrap_or_else(|e| {
                    eprintln!("Saver task error: {:?}", e);
                    std::process::exit(1);
                }),
            }

            std::process::exit(0);
        })
        .await;
}
