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

    pub fn update(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        self.trades.push(trade);
        Ok(())
    }

    pub fn trades(&self) -> &Vec<TradeEvent> {
        &self.trades
    }
}
