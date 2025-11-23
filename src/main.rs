use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use url::Url;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

type Price = String;
type Quantity = String;

#[derive(Debug, Serialize, Deserialize)]
struct DepthUpdateMessage {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename= "s")]
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

#[derive(Debug, Clone)]
struct OrderBook {
    bids: HashMap<String, String>, // price -> quantity
    asks: HashMap<String, String>, // price -> quantity
}

impl OrderBook {
    fn new() -> Self {
        Self {
            bids: HashMap::new(),
            asks: HashMap::new(),
        }
    }

    fn update(&mut self, bids: &Vec<(Price, Quantity)>, asks: &Vec<(Price, Quantity)>) -> Vec<OrderEvent> {
        let mut events = Vec::new();

        // Process bids
        for bid in bids {
            let price = &bid.0;
            let quantity = &bid.1;

            if quantity == "0.00000000" || quantity == "0.00" {
                // Order removal
                if self.bids.remove(price).is_some() {
                    events.push(OrderEvent::Canceled {
                        side: "BID".to_string(),
                        price: price.clone(),
                        quantity: "0".to_string(), // Quantity is 0 for cancellations
                    });
                }
            } else {
                // Order update/addition
                let old_quantity = self.bids.insert(price.clone(), quantity.clone());
                events.push(OrderEvent::Updated {
                    side: "BID".to_string(),
                    price: price.clone(),
                    old_quantity: old_quantity.unwrap_or_else(|| "0".to_string()),
                    new_quantity: quantity.clone(),
                });
            }
        }

        // Process asks
        for ask in asks {
            let price = &ask.0;
            let quantity = &ask.1;

            if quantity == "0.00000000" || quantity == "0.00" {
                // Order removal
                if self.asks.remove(price).is_some() {
                    events.push(OrderEvent::Canceled {
                        side: "ASK".to_string(),
                        price: price.clone(),
                        quantity: "0".to_string(),
                    });
                }
            } else {
                // Order update/addition
                let old_quantity = self.asks.insert(price.clone(), quantity.clone());
                events.push(OrderEvent::Updated {
                    side: "ASK".to_string(),
                    price: price.clone(),
                    old_quantity: old_quantity.unwrap_or_else(|| "0".to_string()),
                    new_quantity: quantity.clone(),
                });
            }
        }

        events
    }
}

#[derive(Debug)]
enum OrderEvent {
    Updated {
        side: String,
        price: String,
        old_quantity: String,
        new_quantity: String,
    },
    Canceled {
        side: String,
        price: String,
        quantity: String,
    },
}

struct BinanceConnector {
    symbol: String,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    order_book: OrderBook,
}

impl BinanceConnector {
    fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_lowercase(),
            reconnect_attempts: 0,
            max_reconnect_attempts: 2,
            order_book: OrderBook::new(),
        }
    }

    async fn connect_websocket(&self) -> Result<
        (
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
                Message
            >,
            futures_util::stream::SplitStream<
                tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
            >
        ),
        Box<dyn std::error::Error>
    > {
        let stream_url = format!(
            "wss://stream.binance.com:9443/ws/{}@depth@100ms",
            self.symbol
        );

        println!("🔗 Подключаемся к: {}", stream_url);

        let url = Url::parse(&stream_url)?;
        let (ws_stream, response) = connect_async(url).await?;

        println!("✅ Подключение установлено! HTTP статус: {}", response.status());
        println!("🎯 Торговая пара: {}", self.symbol.to_uppercase());
        println!("📊 Режим: Level 2 Order Book (обновления каждые 100мс)");
        println!("{}", "=".repeat(80));

        Ok(ws_stream.split())
    }

    fn handle_depth_message(&mut self, text: &str) {
        match serde_json::from_str::<DepthUpdateMessage>(text) {
            Ok(depth_update) => {
                let events = self.order_book.update(&depth_update.bids_to_update, &depth_update.asks_to_update);
                self.print_order_events(&events, &depth_update);
            }
            Err(e) => {
                eprintln!("❌ Не удалось распарсить сообщение: {}\nОшибка: {}", text, e);
            }
        }
    }

    fn print_order_events(&self, events: &[OrderEvent], depth_update: &DepthUpdateMessage) {
        let timestamp = chrono::DateTime::from_timestamp_millis(depth_update.event_time as i64)
            .map(|dt| dt.format("%H:%M:%S%.3f").to_string())
            .unwrap_or_else(|| depth_update.event_time.to_string());

        for event in events {
            match event {
                OrderEvent::Updated { side, price, old_quantity, new_quantity } => {
                    let side_color = if side == "BID" { "\x1b[32m" } else { "\x1b[31m" };
                    let reset_color = "\x1b[0m";

                    let action = if old_quantity == "0" {
                        "CREATED"
                    } else {
                        "UPDATED"
                    };

                    println!(
                        "{} {}{:8}{} {:10} {:14} | {:8} -> {:8}",
                        timestamp,
                        side_color,
                        side,
                        reset_color,
                        action,
                        price,
                        old_quantity,
                        new_quantity
                    );
                }
                OrderEvent::Canceled { side, price, .. } => {
                    let side_color = if side == "BID" { "\x1b[32m" } else { "\x1b[31m" };
                    let reset_color = "\x1b[0m";

                    println!(
                        "{} {}{:8}{} {:10} {:14} | {}",
                        timestamp,
                        side_color,
                        side,
                        reset_color,
                        "CANCELED",
                        price,
                        "REMOVED".to_string()
                    );
                }
            }
        }

        // Показываем статистику по стакану
        if !events.is_empty() {
            println!(
                "📊 Стакан: {} bid / {} ask | Update ID: {}",
                self.order_book.bids.len(),
                self.order_book.asks.len(),
                depth_update.final_update_id
            );
            println!("{}", "-".repeat(80));
        }
    }

    async fn run_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut write, mut read) = self.connect_websocket().await?;

        println!("📊 Ожидаем обновления стакана заявок...\n");
        println!("Легенда:");
        println!("  🟢 BID CREATED - Новая заявка на покупку");
        println!("  🔴 ASK CREATED - Новая заявка на продажу");
        println!("  🟡 BID/ASK UPDATED - Изменение объема существующей заявки");
        println!("  ⚫ BID/ASK CANCELED - Отмена заявки");
        println!("{}", "=".repeat(80));

        loop {
            tokio::select! {
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_depth_message(&text);
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
                                self.handle_depth_message(&text);
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

    pub async fn run(&mut self) {
        loop {
            match self.run_connection().await {
                Ok(_) => {
                    println!("Соединение завершено нормально");
                    break;
                }
                Err(e) => {
                    self.reconnect_attempts += 1;
                    eprintln!("❌ Ошибка подключения: {}", e);

                    if self.reconnect_attempts >= self.max_reconnect_attempts {
                        eprintln!("🚫 Достигнуто максимальное количество попыток переподключения");
                        break;
                    }

                    let delay = 2u64.pow(self.reconnect_attempts);
                    println!("🔄 Переподключение через {} секунд...", delay);
                    sleep(Duration::from_secs(delay)).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 Binance WebSocket Order Book Connector (Level 2)");
    println!("📊 Отслеживание созданных, измененных и отмененных заявок");
    println!("Поддерживаемые пары: btcusdt, ethusdt, adausdt, etc.");
    println!("Для выхода нажмите Ctrl+C\n");

    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let mut connector = BinanceConnector::new(&symbol);
    connector.run().await;
}