use crate::connector::errors::ConnectorError;
use crate::connector::errors::ConnectorError::BuilderError;
use crate::connector::{BinanceConnector, Connector};
use std::any::TypeId;
use crate::connector::connector::ConnectorConfig;

pub struct ConnectorBuilder {
    subscribe_trades: bool,
    subscribe_depth: bool,
    depth_value: u8,
    tickers: Vec<String>,
    errors: Vec<ConnectorError>,
}



impl ConnectorBuilder {
    pub fn new() -> Self {
        Self {
            subscribe_trades: false,
            subscribe_depth: false,
            depth_value: 0,
            tickers: vec![],
            errors: vec![],
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
        let mut config = ConnectorConfig::new(
            1f32,
            1f32,
            self.tickers,
            self.subscribe_trades,
            self.subscribe_depth,
            self.depth_value,

        );
        config.validate()?;

        Ok(())
    }
}
