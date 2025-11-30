use crate::connector::config::{ConnectorConfig, TickerConfig, TickerConfigValidator};
use crate::connector::errors::ConnectorError;
use crate::connector::BinanceConnector;
use crate::shared::Bus;
use std::sync::Arc;

pub struct ConnectorBuilder {
    subscribe_trades: bool,
    subscribe_depth: bool,
    depth_value: u8,
    tickers: Vec<(String, f64, f64)>,
    errors: Vec<ConnectorError>,
    bus: Arc<Bus>,
}

impl ConnectorBuilder {
    pub fn new(bus: Arc<Bus>) -> Self {
        Self {
            subscribe_trades: false,
            subscribe_depth: false,
            depth_value: 0,
            tickers: vec![],
            errors: vec![],
            bus,
        }
    }

    pub fn ticker(mut self, ticker: &str, price_mult: u32, quantity_mult: u32) -> Self {
        self.tickers
            .push((ticker.to_string(), price_mult as f64, quantity_mult as f64));
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
        let mut ticker_configs = Vec::new();
        for (ticker, price_mult, quantity_mult) in self.tickers.iter() {
            let tc = TickerConfig {
                ticker: ticker.clone(),
                price_multiply: *price_mult,
                quantity_multiply: *quantity_mult,
                subscribe_trades: self.subscribe_trades,
                subscribe_depth: self.subscribe_depth,
                depth_value: self.depth_value,
            };
            TickerConfigValidator::new(&tc).validate()?;
            ticker_configs.push(tc);
        }
        let config = ConnectorConfig { ticker_configs };
        Ok(BinanceConnector::new(self.bus.clone(), config))
    }
}
