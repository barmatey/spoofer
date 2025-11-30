use crate::connector::errors::ConnectorError;
use crate::connector::errors::ConnectorError::BuilderError;

pub trait Connector {
    async fn listen(&mut self) {}
}

pub struct ConnectorConfig {
    pub price_multiply: f64,
    pub quantity_multiply: f64,
    pub tickers: Vec<String>,
    pub subscribe_trades: bool,
    pub subscribe_depth: bool,
    pub depth_value: u8,
    errors: Vec<ConnectorError>,
}

impl ConnectorConfig {
    pub fn new(
        price_multiply: f64,
        quantity_multiply: f64,
        tickers: Vec<String>,
        subscribe_trades: bool,
        subscribe_depth: bool,
        depth_value: u8,
    ) -> Self {
        Self {
            price_multiply,
            quantity_multiply,
            tickers,
            subscribe_trades,
            subscribe_depth,
            depth_value,
            errors: vec![],
        }
    }
    fn is_valid_ticker(&self, t: &str) -> bool {
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
    fn validate_tickers(&mut self) {
        if self.tickers.is_empty() {
            let err = BuilderError("At list one ticker required".to_string());
            self.errors.push(err);
        }
        for t in self.tickers.iter() {
            if !self.is_valid_ticker(&t) {
                let err = BuilderError(
                    format!("Ticker should be one of the following formats: AAPL for stocks; BTC/USD for cryptocurrencies. Your value is '{}'",
                            t
                    ));
                self.errors.push(err);
            }
        }
    }

    fn validate_depth(&mut self) {
        if self.subscribe_depth && self.depth_value <= 0 {
            let err = BuilderError("Depth value should be more than 0".to_string());
            self.errors.push(err);
        }
    }

    pub fn validate(&mut self) -> Result<(), ConnectorError> {
        self.validate_depth();
        self.validate_tickers();
        if let Some(e) = self.errors.pop() {
            return Err(e);
        }
        Ok(())
    }
}
