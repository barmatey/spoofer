use crate::connector::errors::{ConnectorError, ParsingError};
use crate::connector::Connector;
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side};
use crate::trade::TradeEvent;

use crate::connector::errors::ParsingError::ConvertingError;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use crate::connector::services::parser::{parse_json, parse_timestamp_from_date_string, parse_value};
use crate::connector::services::websocket::{connect_websocket, send_ws_message, websocket_event_loop, Connection};
// =============================
// Config
// =============================

pub struct KrakenConfig {
    pub ticker: String,
    pub price_multiply: f64,
    pub quantity_multiply: f64,
}

// =============================
// Structure of book/trade data (partial, simplified)
// =============================

#[derive(Debug, Deserialize)]
struct BookSide {
    price: f64,
    qty: f64,
}

#[derive(Debug, Deserialize)]
struct KrakenBookEntry {
    bids: Vec<BookSide>,
    asks: Vec<BookSide>,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct KrakenTrade {
    price: f64,
    qty: f64,
    side: String,
    timestamp: String,
}

// =============================
// Connector
// =============================

pub struct KrakenConnector {
    bus: Arc<Bus>,
    config: KrakenConfig,
}

impl KrakenConnector {
    pub fn new(bus: Arc<Bus>, config: KrakenConfig) -> Self {
        Self { bus, config }
    }

    async fn connect(&self) -> Result<Connection, ConnectorError> {
        let url = "wss://ws.kraken.com/v2";
        println!("[kraken] Connecting to {}", url);
        let (mut write, read) = connect_websocket(url).await?;
        println!("[kraken] Connected");

        // Send subscribe for trades
        let sub_trade = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channel": "trade",
                "symbol": [ self.config.ticker ]
            }
        });
        send_ws_message(&mut write, Message::Text(sub_trade.to_string())).await?;
        println!("[kraken] Sent trade subscribe");

        // Send subscribe for book
        let sub_book = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channel": "book",
                "symbol": [ self.config.ticker ],
                "depth": 10,
                "snapshot": false
            }
        });
        send_ws_message(&mut write, Message::Text(sub_book.to_string())).await?;
        println!("[kraken] Sent book subscribe");

        Ok((write, read))
    }

    fn process_message(&mut self, raw: &str) -> Result<(), ConnectorError> {
        // Try to parse raw JSON
        let v: Value = match serde_json::from_str(raw) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("[kraken] JSON parse error: {:?}, raw: {}", e, raw);
                return Err(ConnectorError::ParsingError(
                    ParsingError::MessageParsingError(format!("JSON parse error: {:?}", e)),
                ));
            }
        };

        // We expect object messages for data (book/trade)
        let obj = match v.as_object() {
            Some(o) => o,
            None => {
                println!("Object is None");
                return Ok(());
            }
        };

        let ch = match obj.get("channel").and_then(|c| c.as_str()) {
            Some(c) => c,
            None => {
                println!("Channel is None");
                return Ok(());
            }
        };

        match ch {
            "book" => self.handle_book(obj)?,
            "trade" => self.handle_trade(obj)?,
            "heartbeat" => { /* ignore */ }
            _ => println!("Unexpected channel {}", ch),
        }


        Ok(())
    }

    fn handle_book(&mut self, obj: &serde_json::Map<String, Value>) -> Result<(), ConnectorError> {
        // Extract data array
        let data = obj
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ParsingError::MessageParsingError("book: missing data array".into()))?;

        for item in data {
            let entry: KrakenBookEntry = parse_json(&item.to_string())?;

            for bid in entry.bids {
                let ts = parse_timestamp_from_date_string(&entry.timestamp)?;
                let price = bid.price * self.config.price_multiply;
                let qty = bid.qty * self.config.quantity_multiply;
                self.bus.levels.publish(LevelUpdated {
                    side: Side::Buy,
                    price: price as Price,
                    quantity: qty as Quantity,
                    timestamp: ts,
                });
            }

            for ask in entry.asks {
                let price = ask.price * self.config.price_multiply;
                let qty = ask.qty * self.config.quantity_multiply;
                let ts = parse_timestamp_from_date_string(&entry.timestamp)?;
                self.bus.levels.publish(LevelUpdated {
                    side: Side::Sell,
                    price: price as Price,
                    quantity: qty as Quantity,
                    timestamp: ts,
                });
            }
        }

        Ok(())
    }

    fn handle_trade(&mut self, obj: &serde_json::Map<String, Value>) -> Result<(), ConnectorError> {
        let data = obj
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ParsingError::MessageParsingError("trade: missing data array".into()))?;

        for item in data {
            let tr: KrakenTrade = parse_value(item.clone())?;

            let price_f = &tr.price * self.config.price_multiply;
            let qty_f = &tr.qty * self.config.quantity_multiply;
            let ts = parse_timestamp_from_date_string(&tr.timestamp)?;

            let side = match tr.side.as_str() {
                "buy" => Side::Buy,
                "sell" => Side::Sell,
                _ => return Err(ConvertingError(format!("Unexpected side {}", tr.side)))?,
            };

            let event = TradeEvent {
                exchange: "kraken".into(),
                price: price_f as Price,
                quantity: qty_f as Quantity,
                timestamp: ts,
                market_maker: side,
            };

            self.bus.trades.publish(event);
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), ConnectorError> {
        let (write, read) = self.connect().await?;
        println!("[kraken] entering ws event loop");
        websocket_event_loop(write, read, |msg| self.process_message(msg)).await?;
        Ok(())
    }
}

impl Connector for KrakenConnector {
    async fn listen(&mut self) {
        if let Err(e) = self.run().await {
            eprintln!("[kraken] error in run: {:?}", e);
        }
    }
}
