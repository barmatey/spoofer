use crate::connector::config::{ConnectorConfig, TickerConfig, TickerConfigValidator};
use crate::connector::errors::ConnectorError;
use crate::connector::{BinanceConnector, KrakenConnector};

pub struct ConnectorBuilder {
    subscribe_trades: bool,
    subscribe_depth: bool,
    depth_value: u8,
    tickers: Vec<(String, f64, f64)>,
}

impl ConnectorBuilder {
    pub fn new() -> Self {
        Self {
            subscribe_trades: false,
            subscribe_depth: false,
            depth_value: 0,
            tickers: vec![],
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

    fn build_config(&mut self) -> Result<ConnectorConfig, ConnectorError> {
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
        let config = ConnectorConfig { ticker_configs };
        Ok(config)
    }

    pub fn build_binance_connector(&mut self) -> Result<BinanceConnector, ConnectorError> {
        let config = self.build_config()?;
        Ok(BinanceConnector::new(config))
    }

    pub fn build_kraken_connector(&mut self) -> Result<KrakenConnector, ConnectorError> {
        let config = self.build_config()?;
        Ok(KrakenConnector::new(config))
    }
}
