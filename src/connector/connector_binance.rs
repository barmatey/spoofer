use crate::connector::errors::{ConnectorError, ParsingError};
use crate::connector::services::{
    connect_websocket, parse_json, parse_number, websocket_event_loop, Connection,
};
use crate::connector::Connector;
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side};
use crate::trade::TradeEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

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

pub struct BinanceConnectorConfig {
    pub ticker: String,
    pub price_multiply: f64,
    pub quantity_multiply: f64,
}

pub struct BinanceConnector {
    bus: Arc<Bus>,
    config: BinanceConnectorConfig,
}

impl<'a> BinanceConnector {
    pub fn new(bus: Arc<Bus>, config: BinanceConnectorConfig) -> Self {
        Self { config, bus }
    }

    fn get_event_from_agg_trade(
        &self,
        trade: AggTradeMessage,
    ) -> Result<TradeEvent, ConnectorError> {
        let price = parse_number(&trade.price)? * self.config.price_multiply;
        let qty = parse_number(&trade.quantity)? * self.config.quantity_multiply;

        let event = TradeEvent {
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

        for (price, quantity) in depth.bids_to_update.iter() {
            let price = parse_number(price)? * self.config.price_multiply;
            let quantity = parse_number(quantity)? * self.config.quantity_multiply;

            result.push(LevelUpdated {
                side: Side::Buy,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: depth.event_time,
            });
        }

        for (price, quantity) in depth.asks_to_update.iter() {
            let price = parse_number(price)? * self.config.price_multiply;
            let quantity = parse_number(quantity)? * self.config.quantity_multiply;

            result.push(LevelUpdated {
                side: Side::Sell,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: depth.event_time,
            });
        }

        Ok(result)
    }

    async fn connect(&self) -> Result<Connection, ConnectorError> {
        let url = format!(
            "wss://stream.binance.com:9443/stream?streams={}@depth@100ms/{}@aggTrade",
            self.config.ticker, self.config.ticker,
        );

        connect_websocket(&url).await.map_err(|e| {
            eprintln!("Failed to connect websocket: {:?}", e);
            ConnectorError::from(e)
        })
    }

    fn process_message(&mut self, txt: &str) -> Result<(), ConnectorError> {
        let wrapper: Value = serde_json::from_str(txt).map_err(|e| {
            ConnectorError::ParsingError(ParsingError::MessageParsingError(format!(
                "Failed to parse wrapper: {:?}, error: {:?}",
                txt, e
            )))
        })?;

        let data = wrapper.get("data").ok_or_else(|| {
            ConnectorError::ParsingError(ParsingError::MessageParsingError(format!(
                "Missing 'data' field in wrapper: {}",
                txt
            )))
        })?;

        let event_type = data.get("e").and_then(|v| v.as_str()).ok_or_else(|| {
            ConnectorError::ParsingError(ParsingError::MessageParsingError(format!(
                "Missing 'e' field in data: {}",
                data
            )))
        })?;

        match event_type {
            "depthUpdate" => self.handle_depth_message(data),
            "aggTrade" => self.handle_agg_trade_message(data),
            other => Err(ConnectorError::ParsingError(
                ParsingError::MessageParsingError(format!("Unknown event type: {}", other)),
            )),
        }
    }

    fn handle_depth_message(&mut self, data: &Value) -> Result<(), ConnectorError> {
        let txt = data.to_string();
        let parsed = parse_json(&txt)?;
        for e in self.get_events_from_depth(parsed)? {
            self.bus.levels.publish(e);
        }
        Ok(())
    }

    fn handle_agg_trade_message(&mut self, data: &Value) -> Result<(), ConnectorError> {
        let txt = data.to_string();
        let msg = parse_json::<AggTradeMessage>(&txt)?;
        let event = self.get_event_from_agg_trade(msg)?;
        self.bus.trades.publish(event);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), ConnectorError> {
        let (write, read) = self.connect().await?;
        websocket_event_loop(write, read, |msg| self.process_message(msg)).await?;
        Ok(())
    }
}

impl Connector for BinanceConnector {
    async fn listen(&mut self) {
        let _ = self.run().await;
    }
}
