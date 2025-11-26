use crate::shared::datetime::now_timestamp;
use crate::shared::{Price, Quantity, TimestampMS};
use crate::trade::TradeEvent;

pub struct TradeStats {
    trades: Vec<TradeEvent>,
    period: TimestampMS,
}

impl TradeStats {
    pub fn new() {}

    fn actualize_trades(&mut self) {
        let threshold = now_timestamp().saturating_sub(self.period);
        let mut i = 0;
        for trade in self.trades.iter() {
            if trade.timestamp < threshold {
                break;
            }
            i += 1;
        }
        self.trades = self.trades[i..].to_vec()
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

    pub fn handle_trade_events(&mut self, events: &[TradeEvent]) {
        for ev in events {
            self.trades.push(ev.clone());
        }
    }
}
