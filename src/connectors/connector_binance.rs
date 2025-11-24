use crate::bus::Bus;
use crate::connectors::Connector;
use crate::events::{LevelUpdated, TradeEvent, Price, Quantity, Side};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

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
    bids_to_update: Vec<(Price, Quantity)>,
    #[serde(rename = "a")]
    asks_to_update: Vec<(Price, Quantity)>,
}

impl DepthUpdateMessage {
    fn process_side(
        &self,
        result: &mut Vec<LevelUpdated>,
        orders: &[(Price, Quantity)],
        side: Side,
    ) {
        for (price, quantity) in orders.iter() {
            result.push(LevelUpdated {
                side: side.clone(),
                price: price.clone(),
                quantity: quantity.clone(),
                timestamp: self.event_time,
            });
        }
    }

    pub fn get_events(&self) -> Vec<LevelUpdated> {
        let mut result = Vec::with_capacity(self.bids_to_update.len() + self.asks_to_update.len());
        self.process_side(&mut result, &self.bids_to_update, Side::Buy);
        self.process_side(&mut result, &self.asks_to_update, Side::Sell);
        result
    }
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
    price: Price,
    #[serde(rename = "q")]
    quantity: Quantity,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
}

impl AggTradeMessage {
    pub fn to_event(&self) -> TradeEvent {
        TradeEvent {
            price: self.price.clone(),
            quantity: self.quantity.clone(),
            timestamp: self.event_time,
            is_buyer_maker: self.is_buyer_maker,
        }
    }
}

pub struct BinanceConnector<'a> {
    ticker: String,
    bus: &'a Bus,
}

impl<'a> BinanceConnector<'a> {
    pub fn new(bus: &'a Bus, ticker: &str) -> Self {
        Self {
            ticker: ticker.to_string(),
            bus,
        }
    }

    async fn handle_depth(&self, txt: &str) {
        if let Ok(msg) = serde_json::from_str::<DepthUpdateMessage>(txt) {
            for e in msg.get_events() {
                self.bus.publish(Arc::new(e));
            }
        }
    }

    async fn handle_trade(&self, txt: &str) {
        if let Ok(msg) = serde_json::from_str::<AggTradeMessage>(txt) {
            self.bus.publish(Arc::new(msg.to_event()));
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
            self.ticker, self.ticker
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
                        // Combined streams wrap data like {"stream":"...","data":{...}}
                        if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(&txt) {
                            if let Some(data) = wrapper.get("data") {
                                let s = data.to_string();
                                self.handle_depth(&s).await;
                                self.handle_trade(&s).await;
                            }
                        }
                    }
                }
                _ = sleep(Duration::from_secs(20)) => {
                    let _ = write.send(Message::Ping(vec![])).await;
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
