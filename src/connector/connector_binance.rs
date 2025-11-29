use crate::connector::errors::{ConnectorError, ParsingError};
use crate::connector::services::{connect_websocket, parse_json, parse_str};
use crate::connector::Connector;
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side};
use crate::trade::TradeEvent;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;

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
        let price = parse_str::<f64>(&trade.price)? * self.config.price_multiply;
        let qty = parse_str::<f64>(&trade.quantity)? * self.config.quantity_multiply;

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
            let price = (parse_str::<f64>(price)? * self.config.price_multiply);
            let quantity = (parse_str::<f64>(quantity)? * self.config.quantity_multiply);

            result.push(LevelUpdated {
                side: Side::Buy,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: depth.event_time,
            });
        }

        for (price, quantity) in depth.asks_to_update.iter() {
            let price = (parse_str::<f64>(price)? * self.config.price_multiply);
            let quantity = (parse_str::<f64>(quantity)? * self.config.quantity_multiply);

            result.push(LevelUpdated {
                side: Side::Sell,
                price: price as Price,
                quantity: quantity as Quantity,
                timestamp: depth.event_time,
            });
        }

        Ok(result)
    }

    fn handle_depth(&mut self, txt: &str) -> Result<(), ConnectorError> {
        let parsed = parse_json(txt)?;
        for e in self.get_events_from_depth(parsed)? {
            self.bus.levels.publish(e);
        }
        Ok(())
    }

    fn handle_agg_trade(&mut self, txt: &str) -> Result<(), ConnectorError> {
        let msg = parse_json::<AggTradeMessage>(txt)?;
        let event = self.get_event_from_agg_trade(msg)?;
        self.bus.trades.publish(event);
        Ok(())
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "wss://stream.binance.com:9443/stream?streams={}@depth@100ms/{}@aggTrade",
            self.config.ticker, self.config.ticker,
        );
        let (mut write, mut read) = connect_websocket(&url).await?;

        loop {
            tokio::select! {
                message = read.next() => {
                    if let Some(Ok(Message::Text(txt))) = message {
                        // Binance combined stream: {"stream":"...","data":{...}}
                        if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(&txt) {
                            if let Some(data) = wrapper.get("data") {
                                if let Some(event_type) = data.get("e").and_then(|v| v.as_str()) {
                                    match event_type {
                                        "depthUpdate" => {
                                            self.handle_depth(&data.to_string());
                                        },
                                        "aggTrade" => {
                                            self.handle_agg_trade(&data.to_string());
                                        },
                                        _ => {}
                                    }
                                }
                            }
                        } else {
                            println!("Failed to parse wrapper: {}", txt);
                        }
                    }
                },
                _ = sleep(Duration::from_secs(20)) => {
                    // Ping для поддержания соединения
                    if let Err(e) = write.send(Message::Ping(vec![])).await {
                        eprintln!("Ping error: {:?}", e);
                    }
                }
            }
        }
    }
}

impl Connector for BinanceConnector {
    async fn listen(&mut self) {
        let _ = self.run().await;
    }
}
