use crate::connector::errors::ConnectorError::{BuilderError};
use crate::connector::errors::{ConnectorError};

use crate::connector::config::{ConnectorConfig, TickerConfig};
use crate::connector::errors::ParsingError::MessageParsingError;
use crate::connector::services::parser::{get_serde_value, parse_json, parse_number};
use crate::connector::services::ticker_map::TickerMap;
use crate::connector::services::websocket::{connect_websocket, Connection};
use crate::connector::{Connector, Event};
use crate::level2::LevelUpdated;
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

fn build_ticker_map(configs: Vec<TickerConfig>) -> TickerMap {
    let mut result = TickerMap::new(convert_ticker_into_binance_symbol);
    for tc in configs {
        result.register(tc);
    }
    result
}

pub struct BinanceUrlBuilder<'a> {
    configs: &'a [TickerConfig],
}

impl<'a> BinanceUrlBuilder<'a> {
    pub fn new(configs: &'a [TickerConfig]) -> Self {
        Self { configs }
    }

    pub fn build_url(&self) -> Result<String, ConnectorError> {
        let streams = self.build_streams()?;
        Ok(format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            streams.join("/")
        ))
    }

    pub fn build_streams(&self) -> Result<Vec<String>, ConnectorError> {
        let mut out = Vec::new();

        for cfg in self.configs {
            let symbol = convert_ticker_into_binance_symbol(&cfg.ticker);
            out.extend(self.build_streams_for_symbol(cfg, &symbol));
        }

        if out.is_empty() {
            return Err(BuilderError(
                "No streams configured. Enable subscribe_trades/subscribe_depth and provide tickers"
                    .to_string(),
            ));
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

    fn build_depth_stream(&self, cfg: &TickerConfig, symbol: &str) -> String {
        if cfg.depth_value > 0 {
            format!("{symbol}@depth{}@100ms", cfg.depth_value)
        } else {
            format!("{symbol}@depth@100ms")
        }
    }

    fn build_trades_stream(&self, symbol: &str) -> String {
        format!("{symbol}@aggTrade")
    }
}

pub struct BinanceConnector {
    configs: TickerMap,
}

impl<'a> BinanceConnector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            configs: build_ticker_map(config.ticker_configs),
        }
    }

    fn get_event_from_agg_trade(
        &self,
        trade: AggTradeMessage,
    ) -> Result<TradeEvent, ConnectorError> {
        let ticker_config = self.configs.get_by_symbol(&trade.symbol.to_lowercase())?;

        let price = parse_number(&trade.price)? * ticker_config.price_multiply;
        let qty = parse_number(&trade.quantity)? * ticker_config.quantity_multiply;

        let event = TradeEvent {
            ticker: ticker_config.ticker.clone(),
            exchange: "binance".to_string(),
            price: price as Price,
            quantity: qty as Quantity,
            timestamp: trade.event_time,
            market_maker: [Side::Sell, Side::Buy][trade.is_buyer_maker as usize],
        };
        Ok(event)
    }

    fn get_events_from_depth(
        &self,
        depth: DepthUpdateMessage,
    ) -> Result<Vec<LevelUpdated>, ConnectorError> {
        let mut result =
            Vec::with_capacity(depth.bids_to_update.len() + depth.asks_to_update.len());

        let ticker_config = self.configs.get_by_symbol(&depth.symbol.to_lowercase())?;

        for (price, quantity) in depth.bids_to_update.iter() {
            let price = parse_number(price)? * ticker_config.price_multiply;
            let quantity = parse_number(quantity)? * ticker_config.quantity_multiply;

            result.push(LevelUpdated {
                exchange: "binance".to_string(),
                ticker: ticker_config.ticker.clone(),
                side: Side::Buy,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: depth.event_time,
            });
        }

        for (price, quantity) in depth.asks_to_update.iter() {
            let price = parse_number(price)? * ticker_config.price_multiply;
            let quantity = parse_number(quantity)? * ticker_config.quantity_multiply;

            result.push(LevelUpdated {
                ticker: ticker_config.ticker.clone(),
                exchange: "binance".to_string(),
                side: Side::Sell,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: depth.event_time,
            });
        }

        Ok(result)
    }

    fn handle_depth_message(&self, data: &Value) -> Result<Vec<Event>, ConnectorError> {
        let txt = data.to_string();
        let parsed = parse_json(&txt)?;
        let events = self
            .get_events_from_depth(parsed)?
            .into_iter()
            .map(|x| Event::LevelUpdate(x))
            .collect();
        Ok(events)
    }

    fn handle_agg_trade_message(&self, data: &Value) -> Result<Vec<Event>, ConnectorError> {
        let txt = data.to_string();
        let msg = parse_json::<AggTradeMessage>(&txt)?;
        let event = self.get_event_from_agg_trade(msg)?;
        Ok(vec![Event::Trade(event)])
    }
}

impl Connector for BinanceConnector {
    async fn connect(&self) -> Result<Connection, ConnectorError> {
        let builder = BinanceUrlBuilder::new(self.configs.get_all_configs());
        let url = builder.build_url()?;

        connect_websocket(&url).await.map_err(|e| {
            eprintln!("Failed to connect websocket: {:?}", e);
            ConnectorError::from(e)
        })
    }

    fn on_message(&self, msg: &str) -> Result<Vec<Event>, ConnectorError> {
        let wrapper = get_serde_value(msg)?;

        let data = wrapper.get("data").ok_or_else(|| {
            MessageParsingError(format!("Missing 'data' field in wrapper: {}", msg))
        })?;

        let event_type = data
            .get("e")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MessageParsingError(format!("Missing 'e' field in data: {}", data)))?;

        match event_type {
            "depthUpdate" => self.handle_depth_message(data),
            "aggTrade" => self.handle_agg_trade_message(data),
            other => Err(MessageParsingError(format!(
                "Unknown event type: {}",
                other
            )))?,
        }
    }
}
