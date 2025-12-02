use crate::connector::config::{ConnectorConfig, TickerConfig, TickerConfigValidator};
use crate::connector::errors::{Error, ErrorHandler};
use crate::connector::{BinanceConnector, KrakenConnector};
use std::sync::Arc;

pub struct ConnectorBuilder {
    subscribe_trades: bool,
    subscribe_depth: bool,
    depth_value: u8,
    tickers: Vec<(String, f64, f64)>,
    error_handlers: Vec<ErrorHandler>,
}

impl ConnectorBuilder {
    pub fn new() -> Self {
        Self {
            subscribe_trades: false,
            subscribe_depth: false,
            depth_value: 0,
            tickers: vec![],
            error_handlers: vec![],
        }
    }

    pub fn add_ticker(mut self, ticker: &str, price_mult: u32, quantity_mult: u32) -> Self {
        self.tickers
            .push((ticker.to_string(), price_mult as f64, quantity_mult as f64));
        self
    }

    pub fn add_error_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&Error) + 'static,
    {
        let boxed = Arc::new(handler);
        self.error_handlers.push(boxed);
        self
    }

    pub fn log_level(mut self, value: &str) -> Self {
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

    fn build_config(&mut self) -> Result<ConnectorConfig, Error> {
        let mut ticker_configs = Vec::new();
        for (ticker, price_multiply, quantity_multiply) in self.tickers.iter() {
            let tc = TickerConfig {
                ticker: ticker.clone(),
                price_multiply: *price_multiply,
                quantity_multiply: *quantity_multiply,
                subscribe_trades: self.subscribe_trades,
                subscribe_depth: self.subscribe_depth,
                depth_value: self.depth_value,
            };
            TickerConfigValidator::new(&tc).validate()?;
            ticker_configs.push(tc);
        }
        let config = ConnectorConfig {
            ticker_configs,
            error_handlers: self.error_handlers.clone(),
        };
        Ok(config)
    }

    pub fn build_binance_connector(&mut self) -> Result<BinanceConnector, Error> {
        let config = self.build_config()?;
        Ok(BinanceConnector::new(config))
    }

    pub fn build_kraken_connector(&mut self) -> Result<KrakenConnector, Error> {
        let config = self.build_config()?;
        Ok(KrakenConnector::new(config))
    }
}
