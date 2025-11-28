use crate::trade::errors::TradeError;
use crate::trade::TradeEvent;

pub struct TradeStore {
    trades: Vec<TradeEvent>,
}

impl TradeStore {
    pub fn new() -> Self {
        Self { trades: Vec::new() }
    }

    pub fn handle_trade(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        if let Some(last) = self.trades.last() {
            if trade.timestamp < last.timestamp {
                return Err(TradeError::OutdatedEvent);
            }
        }
        self.trades.push(trade);
        Ok(())
    }
    
    pub fn trades(&self) -> &Vec<TradeEvent>{
        &self.trades
    }
    
}
