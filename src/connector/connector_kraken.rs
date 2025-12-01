use async_stream::stream;
use crate::connector::errors::{ConnectorError, ParsingError};
use crate::connector::{Connector, Event};
use crate::level2::LevelUpdated;
use crate::shared::{Price, Quantity, Side};
use crate::trade::TradeEvent;

use crate::connector::config::ConnectorConfig;
use crate::connector::errors::ParsingError::{ConvertingError, MessageParsingError};
use crate::connector::services::parser::{
    get_serde_object, parse_json, parse_timestamp_from_date_string, parse_value,
};
use crate::connector::services::ticker_map::TickerMap;
use crate::connector::services::websocket::{connect_websocket, send_ws_message, websocket_event_loop, websocket_stream, Connection};
use futures::Stream;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

pub struct KrakenConfig {
    pub ticker: String,
    pub price_multiply: f64,
    pub quantity_multiply: f64,
}

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
    symbol: String,
}

#[derive(Debug, Deserialize)]
struct KrakenTrade {
    price: f64,
    qty: f64,
    side: String,
    timestamp: String,
    symbol: String,
}

fn build_ticker_map(config: ConnectorConfig) -> TickerMap {
    let mut result = TickerMap::new(|x| x.to_uppercase());
    for x in config.ticker_configs {
        result.register(x);
    }
    result
}

pub struct KrakenConnector {
    configs: TickerMap,
    exchange_name: String,
}

impl KrakenConnector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            configs: build_ticker_map(config),
            exchange_name: "kraken".to_string(),
        }
    }

    async fn connect(&self) -> Result<Connection, ConnectorError> {
        let url = "wss://ws.kraken.com/v2";
        println!("[kraken] Connecting to {}", url);
        let (mut write, read) = connect_websocket(url).await?;
        println!("[kraken] Connected");

        // Send subscribe for trades
        for ticker_config in self.configs.get_all_configs() {
            let symbol = self.configs.get_symbol_from_ticker(&ticker_config.ticker);

            if ticker_config.subscribe_trades {
                let sub_trade = serde_json::json!({
                    "method": "subscribe",
                    "params": {
                        "channel": "trade",
                        "symbol": [ symbol ]
                    }
                });
                send_ws_message(&mut write, Message::Text(sub_trade.to_string())).await?;
                println!("[kraken] Sent trade subscribe for {}", ticker_config.ticker);
            }

            if ticker_config.subscribe_depth {
                let sub_book = serde_json::json!({
                    "method": "subscribe",
                    "params": {
                        "channel": "book",
                        "symbol": [ symbol ],
                        "depth": ticker_config.depth_value,
                        "snapshot": false
                    }
                });
                send_ws_message(&mut write, Message::Text(sub_book.to_string())).await?;
                println!(
                    "[kraken] Sent book subscribe for {} with {} depth",
                    symbol, ticker_config.depth_value
                );
            }
        }

        Ok((write, read))
    }

    fn process_message(&self, raw: &str) -> Result<Vec<Event>, ConnectorError> {
        let obj = get_serde_object(raw)?;

        let channel = obj
            .get("channel")
            .and_then(|c| c.as_str())
            .ok_or_else(|| MessageParsingError("channel is none".to_string()))?;

        let result = match channel {
            "book" => self.handle_book(&obj)?,
            "trade" => self.handle_trade(&obj)?,
            "status" => {
                println!("{}", channel.to_string());
                vec![]
            }
            "heartbeat" => vec![],
            _ => {
                println!("Unexpected channel {}", channel);
                vec![]
            }
        };
        Ok(result)
    }

    fn handle_book(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> Result<Vec<Event>, ConnectorError> {
        println!("handle_book KRAKEN");

        let mut result = Vec::new();

        let data = obj
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ParsingError::MessageParsingError("book: missing data array".into()))?;

        for item in data {
            let entry: KrakenBookEntry = parse_json(&item.to_string())?;

            let config = self.configs.get_by_symbol(&entry.symbol)?;

            for bid in entry.bids {
                let ts = parse_timestamp_from_date_string(&entry.timestamp)?;
                let price = bid.price * config.price_multiply;
                let qty = bid.qty * config.quantity_multiply;
                let event = LevelUpdated {
                    ticker: config.ticker.clone(),
                    exchange: self.exchange_name.clone(),
                    side: Side::Buy,
                    price: price as Price,
                    quantity: qty as Quantity,
                    timestamp: ts,
                };
                result.push(Event::LevelUpdate(event));
            }

            for ask in entry.asks {
                let price = ask.price * config.price_multiply;
                let qty = ask.qty * config.quantity_multiply;
                let ts = parse_timestamp_from_date_string(&entry.timestamp)?;
                let event = LevelUpdated {
                    exchange: self.exchange_name.clone(),
                    ticker: config.ticker.clone(),
                    side: Side::Sell,
                    price: price as Price,
                    quantity: qty as Quantity,
                    timestamp: ts,
                };
                result.push(Event::LevelUpdate(event));
            }
        }

        Ok(result)
    }

    fn handle_trade(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> Result<Vec<Event>, ConnectorError> {
        let mut result = vec![];

        let data = obj
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ParsingError::MessageParsingError("trade: missing data array".into()))?;

        for item in data {
            let tr: KrakenTrade = parse_value(item.clone())?;
            let config = self.configs.get_by_symbol(&tr.symbol)?;

            let price_f = &tr.price * config.price_multiply;
            let qty_f = &tr.qty * config.quantity_multiply;
            let ts = parse_timestamp_from_date_string(&tr.timestamp)?;

            let side = match tr.side.as_str() {
                "buy" => Side::Buy,
                "sell" => Side::Sell,
                _ => return Err(ConvertingError(format!("Unexpected side {}", tr.side)))?,
            };

            let event = TradeEvent {
                ticker: config.ticker.clone(),
                exchange: self.exchange_name.clone(),
                price: price_f as Price,
                quantity: qty_f as Quantity,
                timestamp: ts,
                market_maker: side,
            };

            result.push(Event::Trade(event));
        }

        Ok(result)
    }
    
}

impl Connector for KrakenConnector{
    async fn stream(&self) -> Result<impl Stream<Item=Event>, ConnectorError> {
        let (write, read) = self.connect().await?;
        let ws = websocket_stream(write, read);

        let this = self;
        let s = stream! {
            futures_util::pin_mut!(ws);

            while let Some(msg) = ws.next().await {
                match msg {
                    Ok(txt) => {
                        match this.process_message(&txt) {
                            Ok(events) => {
                                for ev in events {
                                    yield ev;
                                }
                            }
                            Err(err) => {
                                self.on_processing_error(&err);
                                continue;
                            }
                        }
                    }
                    Err(err) => {
                        self.on_processing_error(&err);
                        continue;
                    }
                }
            }
        };
        Ok(s)
    }
}
