use crate::connector::errors::{Error, ErrorHandler};
use reqwest::get;
use std::collections::HashSet;
use std::sync::Arc;
use crate::connector::config::{ConnectorConfig, TickerConfig};
use crate::connector::connector::{ConnectorInternal, StreamBuffer};
use crate::connector::errors::Error::InternalError;
use crate::connector::errors::ExchangeError::BinanceError;
use crate::connector::errors::ParsingError::MessageParsingError;
use crate::connector::services::parser::{model_from_string, parse_number, parse_serde_value};
use crate::connector::services::ticker_map::TickerMap;
use crate::connector::services::websocket::{connect_websocket, Connection};
use crate::connector::Event;
use crate::level2::LevelUpdated;
use crate::shared::logger::Logger;
use crate::shared::{Price, Quantity, Side};
use crate::trade::TradeEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    bids_to_update: Vec<(String, String)>, // Price, Quantity
    #[serde(rename = "a")]
    asks_to_update: Vec<(String, String)>, // Price, Quantity
}

#[derive(Debug, Serialize, Deserialize)]
struct AggTradeMessage {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
}

fn convert_ticker_into_binance_symbol(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphabetic()) // удаляем "/", "-" и всё лишнее
        .flat_map(|c| c.to_lowercase()) // to_lowercase возвращает итератор
        .collect()
}

async fn fetch_binance_symbols() -> Result<HashSet<String>, Error> {
    let url = "https://api.binance.com/api/v3/exchangeInfo";
    let resp: Value = get(url).await?.json().await?;
    let result = resp["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["symbol"].as_str().unwrap().to_lowercase())
        .collect();
    Ok(result)
}

pub struct BinanceUrlBuilder<'a> {
    configs: &'a [TickerConfig],
}

impl<'a> BinanceUrlBuilder<'a> {
    pub fn new(configs: &'a [TickerConfig]) -> Self {
        Self { configs }
    }

    pub fn build_url(&self) -> Result<String, Error> {
        let streams = self.build_streams()?;
        Ok(format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            streams.join("/")
        ))
    }

    fn build_streams(&self) -> Result<Vec<String>, Error> {
        let mut out = Vec::new();

        for cfg in self.configs {
            let symbol = convert_ticker_into_binance_symbol(&cfg.ticker);
            out.extend(self.build_streams_for_symbol(cfg, &symbol));
        }

        if out.is_empty() {
            Err(BinanceError(
                "No streams configured. Enable subscribe_trades/subscribe_depth and provide tickers"
                    .to_string()
            ))?;
        }

        Ok(out)
    }

    fn build_streams_for_symbol(&self, cfg: &TickerConfig, symbol: &str) -> Vec<String> {
        let mut streams = Vec::new();

        if cfg.subscribe_depth {
            streams.push(self.build_depth_stream(cfg, symbol));
        }

        if cfg.subscribe_trades {
            streams.push(self.build_trades_stream(symbol));
        }

        streams
    }

    fn build_depth_stream(&self, _cfg: &TickerConfig, symbol: &str) -> String {
        format!("{symbol}@depth@100ms")
    }

    fn build_trades_stream(&self, symbol: &str) -> String {
        format!("{symbol}@aggTrade")
    }
}

pub struct BinanceConnector {
    exchange: Arc<String>,
    configs: TickerMap,
    logger: Logger,
    error_handlers: Vec<ErrorHandler>,
}

impl BinanceConnector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            configs: TickerMap::from_configs(
                config.ticker_configs,
                convert_ticker_into_binance_symbol,
            ),
            logger: Logger::new("binance", config.log_level),
            exchange: Arc::new("binance".to_string()),
            error_handlers: config.error_handlers,
        }
    }

    async fn check_symbols(&self) -> Result<(), Error> {
        self.logger.info("Check symbols");

        let valid_symbols = fetch_binance_symbols().await?;
        let symbols = self.configs.get_all_symbols();

        if symbols.is_empty() {
            Err(InternalError("Symbols are empty".to_string()))?;
        }

        for s in symbols {
            if !valid_symbols.contains(&s) {
                Err(BinanceError(format!("Symbol {} does not exist", s)))?;
            }
        }
        Ok(())
    }

    fn handle_depth(&self, data: &Value, result: &StreamBuffer) -> Result<(), Error> {
        self.logger.debug("Handle depth_update message");

        let txt = data.to_string();
        let parsed = model_from_string::<DepthUpdateMessage>(&txt)?;

        let ticker_config = self.configs.get_by_symbol(&parsed.symbol.to_lowercase())?;

        for (price, quantity) in parsed.bids_to_update.iter() {
            let price = parse_number(price)? * ticker_config.price_multiply;
            let quantity = parse_number(quantity)? * ticker_config.quantity_multiply;

            let ev = LevelUpdated {
                exchange: Arc::clone(&self.exchange),
                ticker: Arc::clone(&ticker_config.ticker),
                side: Side::Buy,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: parsed.event_time,
            };
            result.push(Event::LevelUpdate(ev));
        }

        for (price, quantity) in parsed.asks_to_update.iter() {
            let price = parse_number(price)? * ticker_config.price_multiply;
            let quantity = parse_number(quantity)? * ticker_config.quantity_multiply;

            let ev = LevelUpdated {
                ticker: Arc::clone(&ticker_config.ticker),
                exchange: Arc::clone(&self.exchange),
                side: Side::Sell,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: parsed.event_time,
            };
            result.push(Event::LevelUpdate(ev));
        }

        Ok(())
    }

    fn handle_trade(&self, data: &Value, result: &StreamBuffer) -> Result<(), Error> {
        self.logger.debug("Handle trade message");

        let txt = data.to_string();
        let trade = model_from_string::<AggTradeMessage>(&txt)?;

        let ticker_config = self.configs.get_by_symbol(&trade.symbol.to_lowercase())?;

        let price = parse_number(&trade.price)? * ticker_config.price_multiply;
        let qty = parse_number(&trade.quantity)? * ticker_config.quantity_multiply;

        let event = TradeEvent {
            ticker: Arc::clone(&ticker_config.ticker),
            exchange: Arc::clone(&self.exchange),
            price: price as Price,
            quantity: qty as Quantity,
            timestamp: trade.event_time,
            market_maker: [Side::Sell, Side::Buy][trade.is_buyer_maker as usize],
        };
        result.push(Event::Trade(event));

        Ok(())
    }
}

impl ConnectorInternal for BinanceConnector {
    async fn connect(&self) -> Result<Connection, Error> {
        self.logger.info("Connecting...");
        let builder = BinanceUrlBuilder::new(self.configs.get_all_configs());
        let url = builder.build_url()?;
        self.check_symbols().await?;
        connect_websocket(&url, &self.logger).await
    }

    fn on_message(&self, msg: &str, result: &StreamBuffer) -> Result<(), Error> {
        let wrapper = parse_serde_value(msg)?;

        let data = wrapper.get("data").ok_or_else(|| {
            MessageParsingError(format!("Missing 'data' field in wrapper: {}", msg))
        })?;

        let event_type = data
            .get("e")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MessageParsingError(format!("Missing 'e' field in data: {}", data)))?;

        match event_type {
            "depthUpdate" => self.handle_depth(data, result),
            "aggTrade" => self.handle_trade(data, result),
            other => Err(MessageParsingError(format!(
                "Unknown errors type: {}",
                other
            )))?,
        }
    }

    fn on_error(&self, err: &Error) {
        let err_message = format!("{:?}", err);
        self.logger.error(&err_message);
        for handler in self.error_handlers.iter() {
            handler(err)
        }
    }
}
