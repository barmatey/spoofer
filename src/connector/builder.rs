use crate::connector::config::{ConnectorConfig, TickerConfig, TickerConfigValidator};
use crate::connector::errors::Error::BuilderError;
use crate::connector::errors::{Error, ErrorHandler};
use crate::connector::{BinanceConnector, Connector, Event, KrakenConnector};
use futures_util::stream::{self, Select, Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use tracing::Level;

#[derive(Clone, PartialEq)]
pub enum Exchange {
    Binance = 0,
    Kraken = 1,
    All = 2,
}

pub struct StreamConnector {
    subscribe_trades: bool,
    subscribe_depth: bool,
    depth_value: u8,
    tickers: Vec<(String, f64, f64)>,
    exchanges: Vec<Exchange>,
    error_handlers: Vec<ErrorHandler>,
    log_level: Level,
}

impl StreamConnector {
    pub fn new() -> Self {
        Self {
            subscribe_trades: false,
            subscribe_depth: false,
            depth_value: 0,
            tickers: vec![],
            error_handlers: vec![],
            exchanges: vec![Exchange::All],
            log_level: Level::INFO,
        }
    }
    fn validate_exchanges(&self) -> Result<(), Error> {
        if self.exchanges.len() == 0 {
            Err(BuilderError("At least one exchange required".to_string()))?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), Error> {
        self.validate_exchanges()?;
        Ok(())
    }
    pub fn exchanges(mut self, value: &[Exchange]) -> Self {
        self.exchanges = value.to_vec();
        self
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

    pub fn log_level_info(mut self) -> Self {
        self.log_level = Level::INFO;
        self
    }

    pub fn log_level_error(mut self) -> Self {
        self.log_level = Level::ERROR;
        self
    }

    pub fn log_level_debug(mut self) -> Self {
        self.log_level = Level::DEBUG;
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

    fn build_config(&self) -> Result<ConnectorConfig, Error> {
        let mut ticker_configs = Vec::new();
        for (ticker, price_multiply, quantity_multiply) in self.tickers.iter() {
            let tc = TickerConfig {
                ticker: Arc::new(ticker.clone()),
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
            log_level: self.log_level,
        };
        Ok(config)
    }

    pub async fn connect(self) -> Result<Pin<Box<dyn Stream<Item = Event>>>, Error> {
        self.validate()?;

        let mut merged: Option<Pin<Box<dyn Stream<Item = Event>>>> = None;

        // Kraken
        if self.exchanges.contains(&Exchange::Kraken) || self.exchanges.contains(&Exchange::All) {
            let config = self.build_config()?;
            let kraken_stream = KrakenConnector::new(config).stream().await?;
            merged = Some(match merged {
                Some(prev) => Box::pin(stream::select(prev, kraken_stream)),
                None => Box::pin(kraken_stream),
            });
        }

        // Binance
        if self.exchanges.contains(&Exchange::Binance) || self.exchanges.contains(&Exchange::All) {
            let config = self.build_config()?;
            let binance_stream = BinanceConnector::new(config).stream().await?;
            merged = Some(match merged {
                Some(prev) => Box::pin(stream::select(prev, binance_stream)),
                None => Box::pin(binance_stream),
            });
        }

        Ok(merged.unwrap())
    }
}
