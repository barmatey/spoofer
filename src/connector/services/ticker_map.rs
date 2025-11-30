use crate::connector::config::TickerConfig;
use crate::connector::errors::ConnectorError;
use crate::connector::errors::ConnectorError::OtherError;
use std::collections::HashMap;

type Converter = fn(&str) -> String;

#[derive(Debug)]
pub struct TickerMap {
    symbols: HashMap<String, usize>,
    tickers: HashMap<String, usize>,
    data: Vec<TickerConfig>,
    converter: Converter,
}

impl TickerMap {
    pub fn new(converter: Converter) -> Self {
        Self {
            symbols: HashMap::new(),
            tickers: HashMap::new(),
            data: Vec::new(),
            converter,
        }
    }

    pub fn register(&mut self, ticker_config: TickerConfig) {
        let symbol = (self.converter)(&ticker_config.ticker);
        let ticker = ticker_config.ticker.clone();

        self.data.push(ticker_config);
        self.tickers.insert(ticker, self.data.len() - 1);
        self.symbols.insert(symbol, self.data.len() - 1);
    }

    pub fn get_by_ticker(&self, ticker: &str) -> Result<&TickerConfig, ConnectorError> {
        let err = || {
            OtherError(format!(
                "Cannot extract linked specific ticker for {}",
                ticker
            ))
        };

        let idx = self.symbols.get(ticker).ok_or_else(err)?;
        self.data.get(*idx).ok_or_else(err)
    }

    pub fn get_by_symbol(&self, symbol: &str) -> Result<&TickerConfig, ConnectorError> {
        let err = || {
            OtherError(format!(
                "Cannot extract linked specific symbol for {}",
                symbol
            ))
        };

        let idx = self.symbols.get(symbol).ok_or_else(err)?;
        self.data.get(*idx).ok_or_else(err)
    }

    pub fn get_all_configs(&self) -> &[TickerConfig] {
        &self.data
    }

    pub fn get_symbol_from_ticker(&self, ticker: &str) -> String{
        (self.converter)(ticker)
    }
}
