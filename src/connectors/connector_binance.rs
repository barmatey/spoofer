use crate::connectors::Connector;
use crate::events::{Price, Quantity};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
struct DepthUpdateMessage {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids_to_update: Vec<(Price, Quantity)>,
    #[serde(rename = "a")]
    asks_to_update: Vec<(Price, Quantity)>,
}

pub struct BinanceConnector {
    ticker: String,
}

impl BinanceConnector {
    pub fn new(ticker: &str) -> Self {
        Self {
            ticker: ticker.to_string(),
        }
    }
    async fn handle_depth_message(&mut self, text: &str) {
        match serde_json::from_str::<DepthUpdateMessage>(text) {
            Ok(depth_update) => {
                println!("{:?}", depth_update);
            }
            Err(e) => {
                eprintln!(
                    "❌ Не удалось распарсить сообщение: {}\nОшибка: {}",
                    text, e
                );
            }
        }
    }
    async fn connect_websocket(&self) -> Result<
        (
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
            futures_util::stream::SplitStream<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        ),
        Box<dyn std::error::Error>,
    > {
        let stream_url = format!(
            "wss://stream.binance.com:9443/ws/{}@depth@100ms",
            self.ticker
        );

        println!("🔗 Подключаемся к: {}", stream_url);

        let url = Url::parse(&stream_url)?;
        let (ws_stream, response) = connect_async(url).await?;

        println!(
            "✅ Подключение установлено! HTTP статус: {}",
            response.status()
        );
        println!("🎯 Торговая пара: {}", self.ticker.to_uppercase());
        println!("📊 Режим: Level 2 Order Book (обновления каждые 100мс)");
        println!("{}", "=".repeat(80));

        Ok(ws_stream.split())
    }

    async fn run_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut write, mut read) = self.connect_websocket().await?;

        println!("📊 Ожидаем обновления стакана заявок...\n");
        println!("{}", "=".repeat(80));

        loop {
            tokio::select! {
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_depth_message(&text).await;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            // Отвечаем на PING для поддержания соединения
                            if let Err(e) = write.send(Message::Pong(data)).await {
                                eprintln!("❌ Ошибка отправки PONG: {}", e);
                                break;
                            }
                        }
                        Some(Ok(Message::Close(frame))) => {
                            if let Some(frame) = frame {
                                println!("🔌 Соединение закрыто: code={}, reason={}", frame.code, frame.reason);
                            } else {
                                println!("🔌 Соединение закрыто сервером");
                            }
                            break;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Игнорируем PONG
                        }
                        Some(Ok(Message::Binary(data))) => {
                            // Пробуем декодировать бинарные данные как текст
                            if let Ok(text) = String::from_utf8(data) {
                                self.handle_depth_message(&text).await;
                            } else {
                                println!("📦 Получено бинарное сообщение");
                            }
                        }
                        Some(Err(e)) => {
                            eprintln!("❌ Ошибка чтения сообщения: {}", e);
                            break;
                        }
                        None => {
                            println!("📴 Соединение разорвано");
                            break;
                        }
                        _ => {}
                    }
                }
                _ = sleep(Duration::from_secs(30)) => {
                    // Отправляем PING каждые 30 секунд для поддержания соединения
                    if let Err(e) = write.send(Message::Ping(vec![])).await {
                        eprintln!("❌ Ошибка отправки PING: {}", e);
                        break;
                    }
                }
            }
        }

        println!("📴 Соединение завершено");
        Ok(())
    }
}

impl Connector for BinanceConnector {
    async fn listen(&mut self) {
        self.run_connection().await;
    }
}
