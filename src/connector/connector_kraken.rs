use crate::connector::errors::{Error, ErrorHandler};
use crate::connector::Event;
use crate::level2::LevelUpdated;
use crate::shared::{Price, Quantity, Side};
use crate::trade::TradeEvent;

use crate::connector::config::{ConnectorConfig};
use crate::connector::connector::{ConnectorInternal, StreamBuffer};
use crate::connector::errors::ExchangeError::KrakenError;
use crate::connector::errors::ParsingError::{ConvertingError, MessageParsingError};
use crate::connector::services::parser::{
    model_from_serde_value, model_from_string, parse_serde_object, parse_timestamp_from_date_string,
};
use crate::connector::services::ticker_map::TickerMap;
use crate::connector::services::websocket::{connect_websocket, send_ws_message, Connection};
use crate::shared::logger::Logger;
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

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

fn convert_ticker_into_kraken_symbol(raw: &str) -> String {
    let result = raw.to_uppercase();
    result
}

fn validate_depth(value: u8) -> Result<(), Error> {
    let available = [10, 25];
    if !available.contains(&value) {
        Err(KrakenError(format!(
            "Depth value must be one of the following numbers: {}",
            available
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))?;
    }
    Ok(())
}

pub struct KrakenConnector {
    configs: TickerMap,
    exchange_name: String,
    logger: Logger,
    error_handlers: Vec<ErrorHandler>,
}

impl KrakenConnector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            configs: TickerMap::from_configs(
                config.ticker_configs,
                convert_ticker_into_kraken_symbol,
            ),
            exchange_name: "kraken".to_string(),
            logger: Logger::new("kraken"),
            error_handlers: config.error_handlers.clone(),
        }
    }

    fn handle_depth(
        &self,
        data: &serde_json::Map<String, Value>,
        result: &mut StreamBuffer,
    ) -> Result<(), Error> {
        let data = data
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| MessageParsingError("book: missing data array".into()))?;

        for item in data {
            let entry: KrakenBookEntry = model_from_string(&item.to_string())?;

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
                result.push_back(Event::LevelUpdate(event));
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
                result.push_back(Event::LevelUpdate(event));
            }
        }

        Ok(())
    }

    fn handle_trade(
        &self,
        obj: &serde_json::Map<String, Value>,
        result: &mut StreamBuffer,
    ) -> Result<(), Error> {
        let data = obj
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| MessageParsingError("trade: missing data array".into()))?;

        for item in data {
            let tr: KrakenTrade = model_from_serde_value(item.clone())?;
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

            result.push_back(Event::Trade(event));
        }

        Ok(())
    }
}

impl ConnectorInternal for KrakenConnector {
    async fn connect(&self) -> Result<Connection, Error> {
        self.logger.info("Connecting...");

        let url = "wss://ws.kraken.com/v2";
        let (mut write, read) = connect_websocket(url, &self.logger).await?;

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
                self.logger.info(&format!(
                    "Sent trade subscribe for {}",
                    ticker_config.ticker
                ));
            }

            if ticker_config.subscribe_depth {
                validate_depth(ticker_config.depth_value)?;
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
                self.logger.info(&format!(
                    "Sent book subscribe for {} with {} depth",
                    symbol, ticker_config.depth_value
                ));
            }
        }

        Ok((write, read))
    }

    fn on_message(&self, msg: &str, buffer: &mut StreamBuffer) -> Result<(), Error> {
        let obj = parse_serde_object(msg)?;

        if let Some(error) = obj.get("error") {
            Err(KrakenError(error.to_string()))?;
        }

        let channel = obj
            .get("channel")
            .and_then(|c| c.as_str())
            .ok_or_else(|| KrakenError("Kraken channel is null".to_string()))?;

        match channel {
            "book" => self.handle_depth(&obj, buffer)?,
            "trade" => self.handle_trade(&obj, buffer)?,
            "status" => {}
            "heartbeat" => {}
            _ => Err(KrakenError(format!("Unexpected channel {}", channel)))?,
        };
        Ok(())
    }

    fn on_error(&self, err: &Error) {
        self.logger.error(&format!("{:?}", err));
        for handler in self.error_handlers.iter() {
            handler(err)
        }
    }
}
