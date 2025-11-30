use crate::connector::errors::ConnectorError;
use crate::connector::errors::ConnectorError::OtherError;
use crate::connector::BinanceConnector;
use crate::shared::Bus;
use std::sync::Arc;

pub struct ConnectorBuilder {
    price_multiply: u32,
    quantity_multiply: u32,
    subscribe_trades: bool,
    subscribe_depth: bool,
    depth_value: u8,
    tickers: Vec<String>,
    errors: Vec<ConnectorError>,
    bus: Arc<Bus>,
}

impl ConnectorBuilder {
    pub fn new(bus: Arc<Bus>) -> Self {
        Self {
            price_multiply: 1,
            quantity_multiply: 1,
            subscribe_trades: false,
            subscribe_depth: false,
            depth_value: 0,
            tickers: vec![],
            errors: vec![],
            bus,
        }
    }

    pub fn tickers(mut self, value: &[&str]) -> Self {
        for &t in value {
            self.tickers.push(t.to_string());
        }
        self
    }

    pub fn subscribe_trades(mut self) -> Self {
        self.subscribe_trades = true;
        self
    }

    pub fn subscribe_depth(mut self, value: u8) -> Self {
        self.subscribe_depth = true;
        self.depth_value = value;
        self
    }
    pub fn build_binance_connector(&mut self) -> Result<BinanceConnector, ConnectorError> {
        Err(OtherError("".to_string()))
    }
}
