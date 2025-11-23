use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use url::Url;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct TradeMessage {
    e: String,  // Event type
    #[serde(rename = "E")]
    event_time: u64,     // Event time
    s: String,  // Symbol
    t: u64,     // Trade ID
    p: String,  // Price
    q: String,  // Quantity
    b: u64,     // Buyer order ID
    a: u64,     // Seller order ID
    #[serde(rename = "T")]
    trade_time: u64,     // Trade time
    m: bool,    // Is the buyer the market maker?
    #[serde(rename = "M")]
    ignore: bool,    // Ignore
}

struct BinanceConnector {
    symbol: String,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
}

impl BinanceConnector {
    fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_lowercase(),
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
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
            "wss://stream.binance.com:9443/ws/{}@trade",
            self.symbol
        );

        println!("🔗 Подключаемся к: {}", stream_url);

        let url = Url::parse(&stream_url)?;
        let (ws_stream, response) = connect_async(url).await?;

        println!("✅ Подключение установлено! HTTP статус: {}", response.status());
        println!("🎯 Торговая пара: {}", self.symbol.to_uppercase());
        println!("{}", "=".repeat(60));

        Ok(ws_stream.split())
    }

    fn handle_trade_message(&self, text: &str) {
        // Пробуем распарсить как прямой trade message
        if let Ok(trade) = serde_json::from_str::<TradeMessage>(text) {
            self.print_trade(&trade);
            return;
        }

        // Пробуем альтернативные форматы
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text) {
            // Формат с полем "data" (combined streams)
            if let Some(data_value) = json_value.get("data") {
                if let Ok(trade) = serde_json::from_value::<TradeMessage>(data_value.clone()) {
                    self.print_trade(&trade);
                    return;
                }
            }

            // Прямой парсинг полей
            if let (Some(s), Some(p), Some(q), Some(m), Some(t)) = (
                json_value.get("s").and_then(|v| v.as_str()),
                json_value.get("p").and_then(|v| v.as_str()),
                json_value.get("q").and_then(|v| v.as_str()),
                json_value.get("m").and_then(|v| v.as_bool()),
                json_value.get("T").and_then(|v| v.as_u64()),
            ) {
                let trade = TradeMessage {
                    e: "trade".to_string(),
                    event_time: 0,
                    s: s.to_string(),
                    t: 0,
                    p: p.to_string(),
                    q: q.to_string(),
                    b: 0,
                    a: 0,
                    trade_time: t,
                    m: m,
                    ignore: false,
                };
                self.print_trade(&trade);
                return;
            }
        }

        eprintln!("❌ Не удалось распарсить сообщение: {}", text);
    }

    fn print_trade(&self, trade: &TradeMessage) {
        let side = if trade.m { "SELL" } else { "BUY" };
        let side_color = if trade.m { "\x1b[31m" } else { "\x1b[32m" };
        let reset_color = "\x1b[0m";

        let timestamp = chrono::DateTime::from_timestamp_millis(trade.trade_time as i64)
            .map(|dt| dt.format("%H:%M:%S%.3f").to_string())
            .unwrap_or_else(|| trade.trade_time.to_string());

        println!(
            "{} {}{:8}{} {:10} {:14} x {:14}",
            timestamp,
            side_color,
            side,
            reset_color,
            trade.s.to_uppercase(),
            trade.q,
            trade.p
        );
    }

    async fn run_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut write, mut read) = self.connect_websocket().await?;

        println!("📊 Ожидаем трейды...\n");

        loop {
            tokio::select! {
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_trade_message(&text);
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
                            println!("📦 Получено бинарное сообщение: {} байт", data.len());
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
    println!("🚀 Binance WebSocket Trade Connector (Асинхронная версия)");
    println!("Поддерживаемые пары: btcusdt, ethusdt, adausdt, etc.");
    println!("Для выхода нажмите Ctrl+C\n");

    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let mut connector = BinanceConnector::new(&symbol);
    connector.run().await;
}