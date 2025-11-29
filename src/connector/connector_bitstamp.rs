use crate::connector::services::{
    connect_websocket, parse_json, parse_number, parse_timestamp, send_ws_message, Connection,
};
use crate::connector::{
    errors::{ConnectorError, ParsingError},
    Connector,
};
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side, TimestampMS};
use crate::trade::TradeEvent;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Serialize, Deserialize)]
struct BitDepth {
    timestamp: String,
    microtimestamp: String,
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
    type_: u8,
}

pub struct BitstampConnectorConfig {
    pub ticker: String,
    pub price_multiply: f64,
    pub quantity_multiply: f64,
}

pub struct BitstampConnector {
    bus: Arc<Bus>,
    config: BitstampConnectorConfig,
}

impl BitstampConnector {
    pub fn new(bus: Arc<Bus>, config: BitstampConnectorConfig) -> Self {
        Self { bus, config }
    }

    fn get_event_from_trade(&self, trade: BitstampTrade) -> Result<TradeEvent, ConnectorError> {
        let event = TradeEvent {
            exchange: "bitstamp".to_string(),
            price: (trade.price * self.config.price_multiply) as Price,
            quantity: (trade.amount * self.config.quantity_multiply) as Quantity,
            timestamp: parse_timestamp(&trade.microtimestamp)? / 1000,
            market_maker: [Side::Buy, Side::Sell][trade.type_ as usize],
        };
        Ok(event)
    }

    fn get_events_from_depth(&self, ob: BitDepth) -> Result<Vec<LevelUpdated>, ConnectorError> {
        let mut result = Vec::with_capacity(ob.bids.len() + ob.asks.len());
        let ts = ob.microtimestamp.parse::<TimestampMS>().unwrap_or(0) / 1000;

        for (price, qty) in ob.bids {
            result.push(LevelUpdated {
                side: Side::Buy,
                price: (parse_number(&price)? * self.config.price_multiply) as Price,
                quantity: (parse_number(&qty)? * self.config.quantity_multiply) as Quantity,
                timestamp: ts,
            });
        }

        for (price, qty) in ob.asks {
            result.push(LevelUpdated {
                price: (parse_number(&price)? * self.config.price_multiply) as Price,
                quantity: (parse_number(&qty)? * self.config.quantity_multiply) as Quantity,
                timestamp: ts,
                side: Side::Sell,
            });
        }

        Ok(result)
    }

    fn process_message(&mut self, txt: &str) -> Result<(), ConnectorError> {
        let wrapper: Value = parse_json(txt)?;

        let event_type = wrapper
            .get("event")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ConnectorError::ParsingError(ParsingError::MessageParsingError(format!(
                    "Missing 'event' in wrapper: {}",
                    txt
                )))
            })?;

        let channel = wrapper
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let data = wrapper.get("data").ok_or_else(|| {
            ConnectorError::ParsingError(ParsingError::MessageParsingError(format!(
                "Missing 'data' in wrapper: {}",
                txt
            )))
        })?;

        match (event_type, channel) {
            ("data", c) if c.starts_with("order_book") => {
                let ob: BitDepth = parse_json(&data.to_string())?;
                for e in self.get_events_from_depth(ob)? {
                    self.bus.levels.publish(e);
                }
            }
            ("data", c) if c.starts_with("live_trades") => {
                let trade: BitstampTrade = parse_json(&data.to_string())?;
                let event = self.get_event_from_trade(trade)?;
                self.bus.trades.publish(event);
            }
            _ => {}
        }

        Ok(())
    }

    async fn connect(&self) -> Result<Connection, ConnectorError> {
        let url = "wss://ws.bitstamp.net".to_string();
        connect_websocket(&url).await
    }

    pub async fn run(&mut self) -> Result<(), ConnectorError> {
        let (mut write, mut read) = self.connect().await?;

        let subscribe_depth = serde_json::json!({
            "event": "bts:subscribe",
            "data": { "channel": format!("order_book_{}", self.config.ticker) }
        });
        let msg = Message::Text(subscribe_depth.to_string());
        send_ws_message(&mut write, msg).await?;

        let subscribe_trades = serde_json::json!({
            "event": "bts:subscribe",
            "data": { "channel": format!("live_trades_{}", self.config.ticker) }
        });
        let msg = Message::Text(subscribe_trades.to_string());
        send_ws_message(&mut write, msg).await?;

        loop {
            tokio::select! {
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(txt))) => {
                            if let Err(err) = self.process_message(&txt) {
                                eprintln!("Failed to process message: {:?}, error: {:?}", txt, err);
                            }
                        }
                        Some(Ok(msg)) => eprintln!("Ignoring non-text message: {:?}", msg),
                        Some(Err(err)) => eprintln!("WebSocket read error: {:?}", err),
                        None => {
                            eprintln!("WebSocket closed, reconnecting...");
                            match self.connect().await {
                                Ok(conn) => {
                                    write = conn.0;
                                    read = conn.1;
                                    eprintln!("Reconnected successfully");
                                }
                                Err(err) => {
                                    eprintln!("Failed to reconnect: {:?}", err);
                                    sleep(Duration::from_secs(5)).await;
                                }
                            }
                        }
                    }
                }
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
