use crate::shared::datetime::now_timestamp;
use crate::shared::{Price, Quantity, TimestampMS};
use crate::trade::errors::TradeError;
use crate::trade::TradeEvent;

pub struct TradeStats {
    trades: Vec<TradeEvent>,
    period: TimestampMS,
}

impl TradeStats {
    pub fn new() {}

    fn actualize_trades(&mut self) {
        let threshold = now_timestamp().saturating_sub(self.period);
        if let Some(idx) = self.trades.iter().position(|t| t.timestamp < threshold) {
            self.trades = self.trades.split_off(idx);
        }
    }

    pub fn get_traded_quantity(&self, price: Price) -> Quantity {
        todo!()
    }

    pub fn get_min_price(&self) -> Price {
        todo!()
    }

    pub fn get_max_price(&self) -> Price {
        todo!()
    }

    pub fn handle_trade_events(&mut self, events: &[TradeEvent]) -> Result<(), TradeError> {
        for ev in events {
            if ev.timestamp < self.trades.last().map(|x| x.timestamp).unwrap_or(0) {
                return Err(TradeError::TimestampError);
            }
            self.trades.push(ev.clone());
        }
        Ok(())
    }
}
