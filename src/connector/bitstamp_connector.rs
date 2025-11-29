use crate::connector::services::connect_websocket;
use crate::connector::Connector;
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side, TimestampMS};
use crate::trade::TradeEvent;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
struct BitstampOrderBook {
    timestamp: String,      // в секундах
    microtimestamp: String, // в микросекундах
    bids: Vec<(String, String)>,
    asks: Vec<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BitstampTrade {
    price: f64,
    amount: f64,
    timestamp: String,
    microtimestamp: String,
    #[serde(rename = "type")]
    type_: u8, // 0 = buy, 1 = sell
}

pub struct BitstampConnectorConfig {
    pub ticker: String,
    pub price_multiply: u32,
    pub quantity_multiply: u32,
}

pub struct BitstampConnector {
    bus: Arc<Bus>,
    config: BitstampConnectorConfig,
    ws_url: String,
}

impl BitstampConnector {
    pub fn new(bus: Arc<Bus>, config: BitstampConnectorConfig) -> Self {
        Self {
            bus,
            config,
            ws_url: "wss://ws.bitstamp.net".to_string(),
        }
    }

    fn get_event_from_trade(&self, trade: BitstampTrade) -> TradeEvent {
        TradeEvent {
            exchange: "bitstamp".to_string(),
            price: (trade.price * self.config.price_multiply as f64) as Price,
            quantity: (trade.amount * self.config.quantity_multiply as f64) as Quantity,
            timestamp: trade.microtimestamp.parse::<u64>().unwrap_or(0) / 1000,
            market_maker: if trade.type_ == 0 {
                Side::Buy
            } else {
                Side::Sell
            },
        }
    }

    fn get_events_from_orderbook(&self, ob: BitstampOrderBook) -> Vec<LevelUpdated> {
        let mut result = Vec::with_capacity(ob.bids.len() + ob.asks.len());
        let ts = ob.microtimestamp.parse::<TimestampMS>().unwrap() / 1000;

        for (price, qty) in ob.bids {
            result.push(LevelUpdated {
                side: Side::Buy,
                price: (price.parse::<f32>().unwrap() * self.config.price_multiply as f32) as Price,
                quantity: (qty.parse::<f32>().unwrap() * self.config.quantity_multiply as f32)
                    as Quantity,
                timestamp: ts,
            });
        }

        for (price, qty) in ob.asks {
            result.push(LevelUpdated {
                side: Side::Sell,
                price: (price.parse::<f32>().unwrap() * self.config.price_multiply as f32) as Price,
                quantity: (qty.parse::<f32>().unwrap() * self.config.quantity_multiply as f32)
                    as Quantity,
                timestamp: ts,
            });
        }

        result
    }

    fn handle_orderbook(&mut self, txt: &str) {
        match serde_json::from_str::<BitstampOrderBook>(txt) {
            Ok(ob) => {
                for e in self.get_events_from_orderbook(ob) {
                    self.bus.levels.publish(e);
                }
            }
            Err(err) => println!("OrderBook parsing error: {:?}", err),
        }
    }

    fn handle_trade(&mut self, txt: &str) {
        match serde_json::from_str::<BitstampTrade>(txt) {
            Ok(trade) => {
                let event = self.get_event_from_trade(trade);
                self.bus.trades.publish(event);
            }
            Err(err) => println!("Trade parsing error: {:?}", err),
        }
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut write, mut read) = connect_websocket(&self.ws_url).await?;

        // Подписка на orderbook
        let subscribe_orderbook = serde_json::json!({
            "event": "bts:subscribe",
            "data": { "channel": format!("order_book_{}", self.config.ticker) }
        });
        write
            .send(Message::Text(subscribe_orderbook.to_string()))
            .await?;

        // Подписка на live trades
        let subscribe_trades = serde_json::json!({
            "event": "bts:subscribe",
            "data": { "channel": format!("live_trades_{}", self.config.ticker) }
        });
        write
            .send(Message::Text(subscribe_trades.to_string()))
            .await?;

        loop {
            tokio::select! {
                message = read.next() => {
                    if let Some(Ok(Message::Text(txt))) = message {
                        if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(&txt) {
                            if let Some(event_type) = wrapper.get("event").and_then(|v| v.as_str()) {
                                if event_type == "data" || event_type == "trade" {
                                    if let Some(channel) = wrapper.get("channel").and_then(|v| v.as_str()) {
                                        match channel {
                                            c if c.starts_with("order_book") => {
                                                self.handle_orderbook(&wrapper["data"].to_string());
                                            }
                                            c if c.starts_with("live_trades") => {
                                                self.handle_trade(&wrapper["data"].to_string());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                _ = sleep(Duration::from_secs(20)) => {
                    if let Err(e) = write.send(Message::Ping(vec![])).await {
                        eprintln!("Ping error: {:?}", e);
                    }
                }
            }
        }
    }
}

impl Connector for BitstampConnector {
    async fn listen(&mut self) {
        let _ = self.run().await;
    }
}
