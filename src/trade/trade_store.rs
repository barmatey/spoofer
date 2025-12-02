use crate::shared::errors::{check_exchange, check_ticker, check_timestamp};
use crate::shared::TimestampMS;
use crate::trade::errors::TradeError;
use crate::trade::TradeEvent;

pub struct TradeStore {
    exchange: String,
    ticker: String,
    trades: Vec<TradeEvent>,
    last_ts: TimestampMS,
}

impl TradeStore {
    pub fn new(exchange: &str, ticker: &str) -> Self {
        Self {
            trades: Vec::new(),
            exchange: exchange.to_string(),
            ticker: ticker.to_string(),
            last_ts: 0,
        }
    }
    pub fn update(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        check_timestamp(self.last_ts, trade.timestamp)?;
        check_exchange(&trade.exchange, &self.exchange)?;
        check_ticker(&trade.ticker, &self.ticker)?;
        self.last_ts = trade.timestamp;
        self.trades.push(trade);
        Ok(())
    }
    pub fn update_if_instrument_matches(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        if self.ticker == trade.ticker && self.exchange == trade.exchange {
            self.update(trade)?;
        }
        Ok(())
    }
    pub fn trades(&self) -> &Vec<TradeEvent> {
        &self.trades
    }
}
