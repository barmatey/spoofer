use crate::connector::errors::Error;
use crate::connector::errors::Error::BuilderError;

#[derive(Debug)]
pub struct TickerConfig {
    pub ticker: String,
    pub price_multiply: f64,
    pub quantity_multiply: f64,
    pub subscribe_trades: bool,
    pub subscribe_depth: bool,
    pub depth_value: u8,
}

pub struct TickerConfigValidator<'a> {
    ticker: &'a TickerConfig,
    errors: Vec<Error>,
}
impl<'a> TickerConfigValidator<'a> {
    pub fn new(ticker: &'a TickerConfig) -> Self {
        Self {
            ticker,
            errors: vec![],
        }
    }
    fn is_valid_symbol(&self, t: &str) -> bool {
        // ABC
        if t.len() >= 1 && t.len() <= 10 && t.chars().all(|c| c.is_ascii_alphabetic()) {
            return true;
        }

        if let Some((base, quote)) = t.split_once('/') {
            if base.len() >= 1
                && base.len() <= 10
                && quote.len() >= 1
                && quote.len() <= 10
                && base.chars().all(|c| c.is_ascii_alphabetic())
                && quote.chars().all(|c| c.is_ascii_alphabetic())
            {
                return true;
            }
        }

        false
    }
    fn validate_symbol(&mut self) {
        if !self.is_valid_symbol(&self.ticker.ticker) {
            let err = BuilderError(
                format!("Ticker should be one of the following formats: AAPL for stocks; BTC/USD for cryptocurrencies. Your value is '{}'",
                        self.ticker.ticker
                ));
            self.errors.push(err);
        }
    }

    fn validate_depth(&mut self) {
        if self.ticker.subscribe_depth && self.ticker.depth_value <= 0 {
            let err = BuilderError("Depth value should be more than 0".to_string());
            self.errors.push(err);
        }
    }

    pub fn validate(&mut self) -> Result<(), Error> {
        self.validate_depth();
        self.validate_symbol();
        if let Some(e) = self.errors.pop() {
            return Err(e);
        }
        Ok(())
    }
}

pub struct ConnectorConfig {
    pub ticker_configs: Vec<TickerConfig>,
}
