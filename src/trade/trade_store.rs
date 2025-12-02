use crate::level2::Level2Error::IncompatibleExchange;
use crate::level2::{Level2Error, LevelUpdated};
use crate::trade::errors::TradeError;
use crate::trade::TradeEvent;

pub struct TradeStore {
    exchange: String,
    ticker: String,
    trades: Vec<TradeEvent>,
}

impl TradeStore {
    pub fn new(exchange: &str, ticker: &str) -> Self {
        Self {
            trades: Vec::new(),
            exchange: exchange.to_string(),
            ticker: ticker.to_string(),
        }
    }

    fn check_timestamp(&self, trade: &TradeEvent) -> Result<(), TradeError> {
        if let Some(last) = self.trades.last() {
            if trade.timestamp < last.timestamp {
                return Err(TradeError::OutdatedEvent(
                    "You are trying to add an errors that earliest last one".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn check_exchange(&self, event: &LevelUpdated) -> Result<(), Level2Error> {
        if event.exchange != self.exchange {
            Err(IncompatibleExchange(format!(
                "TradeStore.exchange != errors.exchange. {} != {}",
                self.exchange, event.exchange
            )))?;
        }
        Ok(())
    }

    fn check_ticker(&self, event: &LevelUpdated) -> Result<(), Level2Error> {
        if event.ticker != self.ticker {
            Err(IncompatibleExchange(format!(
                "TradeStore.ticker != errors.ticker. {} != {}",
                self.ticker, event.ticker
            )))?;
        }
        Ok(())
    }

    pub fn handle_trade(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        self.trades.push(trade);
        Ok(())
    }

    pub fn trades(&self) -> &Vec<TradeEvent> {
        &self.trades
    }
}
