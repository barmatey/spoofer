use crate::connector::Connector;
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side};
use crate::trade::TradeEvent;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

//
// Incoming messages
//

#[derive(Debug, Serialize, Deserialize)]
struct CoinbaseL2Update {
    #[serde(rename = "type")]
    event_type: String,
    product_id: String,
    changes: Vec<(String, String, String)>, // [ "buy"/"sell", price, qty ]
}

#[derive(Debug, Serialize, Deserialize)]
struct CoinbaseTicker {
    #[serde(rename = "type")]
    event_type: String,
    product_id: String,
    price: String,
    last_size: String,
    side: String, // buy/sell
}

//
// Config
//

pub struct CoinbaseConnectorConfig {
    pub product_id: String, // e.g. "BTC-USD"
    pub price_multiply: u32,
    pub quantity_multiply: u32,
}

//
// Connector
//

pub struct CoinbaseConnector<'a> {
    bus: &'a Bus,
    config: CoinbaseConnectorConfig,
}

impl<'a> CoinbaseConnector<'a> {
    pub fn new(bus: &'a Bus, config: CoinbaseConnectorConfig) -> Self {
        Self { config, bus }
    }

    //
    // Parsers
    //

    fn parse_trade(&self, t: CoinbaseTicker) -> TradeEvent {
        let price = (t.price.parse::<f32>().unwrap() * self.config.price_multiply as f32) as Price;
        let qty = (t.last_size.parse::<f32>().unwrap() * self.config.quantity_multiply as f32)
            as Quantity;

        TradeEvent {
            price,
            quantity: qty,
            timestamp: 0, // Coinbase не даёт timestamp в ms
            market_maker: match t.side.as_str() {
                "buy" => Side::Buy,
                "sell" => Side::Sell,
                _ => panic!(),
            },
        }
    }

    fn parse_l2update(&self, update: CoinbaseL2Update) -> Vec<LevelUpdated> {
        let mut out = Vec::with_capacity(update.changes.len());

        for (side, price, size) in update.changes {
            out.push(LevelUpdated {
                side: match side.as_str() {
                    "buy" => Side::Buy,
                    _ => Side::Sell,
                },
                price: (price.parse::<f32>().unwrap() * self.config.price_multiply as f32) as Price,
                quantity: (size.parse::<f32>().unwrap() * self.config.quantity_multiply as f32)
                    as Quantity,
                timestamp: 0,
            });
        }

        out
    }

    //
    // Websocket connect
    //

    async fn connect_websocket(
        &self,
    ) -> Result<
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
        let url = Url::parse("wss://ws-feed.exchange.coinbase.com")?;
        println!("🔗 Connecting to Coinbase: {}", url);

        let (ws, _) = connect_async(url).await?;
        Ok(ws.split())
    }

    //
    // Core loop
    //

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut write, mut read) = self.connect_websocket().await?;

        //
        // Subscribe (fix: выполняем здесь, а не в отдельной async fn)
        //
        let sub = serde_json::json!({
            "type": "subscribe",
            "product_ids": [self.config.product_id],
            "channels": [
                "ticker",
                { "name": "level2", "product_ids": [self.config.product_id] }
            ]
        });

        write.send(Message::Text(sub.to_string())).await?;

        //
        // Main loop
        //
        loop {
            tokio::select! {
                msg = read.next() => {
                    if let Some(Ok(Message::Text(txt))) = msg {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&txt) {
                            if let Some(t) = json.get("type").and_then(|v| v.as_str()) {
                                match t {
                                    "l2update" => {
                                        if let Ok(update) = serde_json::from_str::<CoinbaseL2Update>(&txt) {
                                            for ev in self.parse_l2update(update) {
                                                self.bus.levels.publish(ev);
                                            }
                                        }
                                    }

                                    "ticker" => {
                                        if let Ok(tick) = serde_json::from_str::<CoinbaseTicker>(&txt) {
                                            let ev = self.parse_trade(tick);
                                            self.bus.trades.publish(ev);
                                        }
                                    }

                                    _ => {}
                                }
                            }
                        }
                    }
                }

                // Keepalive ping
                _ = sleep(Duration::from_secs(20)) => {
                    let _ = write.send(Message::Ping(vec![])).await;
                }
            }
        }
    }
}

impl<'a> Connector for CoinbaseConnector<'a> {
    async fn listen(&mut self) {
        let _ = self.run().await;
    }
}
