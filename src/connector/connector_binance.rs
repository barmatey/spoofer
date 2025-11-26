use crate::connector::Connector;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use crate::level2::LevelUpdated;
use crate::shared::{Bus, Price, Quantity, Side};
use crate::trade::TradeEvent;

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
    pub price_multiply: u32,
    pub quantity_multiply: u32,
}

pub struct BinanceConnector<'a> {
    bus: &'a Bus,
    config: BinanceConnectorConfig,
}

impl<'a> BinanceConnector<'a> {
    pub fn new(bus: &'a Bus, config: BinanceConnectorConfig) -> Self {
        Self { config, bus }
    }

    fn get_event_from_agg_trade(&self, trade: AggTradeMessage) -> TradeEvent {
        TradeEvent {
            price: (trade.price.parse::<f32>().unwrap() * self.config.price_multiply as f32)
                as Price,
            quantity: (trade.quantity.parse::<f32>().unwrap()
                * self.config.quantity_multiply as f32) as Quantity,
            timestamp: trade.event_time,
            is_buyer_maker: trade.is_buyer_maker,
        }
    }

    fn get_events_from_depth(&self, depth: DepthUpdateMessage) -> Vec<LevelUpdated> {
        let mut result =
            Vec::with_capacity(depth.bids_to_update.len() + depth.asks_to_update.len());

        for (price, quantity) in depth.bids_to_update.iter() {
            result.push(LevelUpdated {
                side: Side::Buy,
                price: (price.parse::<f32>().unwrap() * self.config.price_multiply as f32) as Price,
                quantity: (quantity.parse::<f32>().unwrap() * self.config.quantity_multiply as f32)
                    as Quantity,
                timestamp: depth.event_time,
            });
        }

        for (price, quantity) in depth.asks_to_update.iter() {
            result.push(LevelUpdated {
                side: Side::Sell,
                price: (price.parse::<f32>().unwrap() * self.config.price_multiply as f32) as Price,
                quantity: (quantity.parse::<f32>().unwrap() * self.config.quantity_multiply as f32)
                    as Quantity,
                timestamp: depth.event_time,
            });
        }

        result
    }

    async fn handle_depth(&mut self, txt: &str) {
        let parsed = serde_json::from_str::<DepthUpdateMessage>(txt);
        match parsed {
            Ok(value) => {
                for e in self.get_events_from_depth(value) {
                    self.bus.levels.publish(e);
                }
            }
            Err(err) => println!("DepthUpdateMessage parsing error: {:?}", err),
        }
    }

    async fn handle_trade(&mut self, txt: &str) {
        match serde_json::from_str::<AggTradeMessage>(txt) {
            Ok(msg) => {
                let event = self.get_event_from_agg_trade(msg);
                self.bus.trades.publish(event);
            }
            Err(err) => println!("AggTradeMessage parsing error: {:?}", err),
        }
    }

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
        let url = format!(
            "wss://stream.binance.com:9443/stream?streams={}@depth@100ms/{}@aggTrade",
            self.config.ticker, self.config.ticker,
        );

        println!("🔗 Connecting: {}", url);

        let (ws_stream, _) = connect_async(Url::parse(&url)?).await?;
        Ok(ws_stream.split())
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut write, mut read) = self.connect_websocket().await?;

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
                                            self.handle_depth(&data.to_string()).await;
                                        },
                                        "aggTrade" => {
                                            self.handle_trade(&data.to_string()).await;
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

impl<'a> Connector for BinanceConnector<'a> {
    async fn listen(&mut self) {
        let _ = self.run().await;
    }
}
