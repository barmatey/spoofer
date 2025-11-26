use crate::trade::TradeEvent;

pub struct TradeStats {
    trades: Vec<TradeEvent>,
}

impl TradeStats {
    pub fn new() {}

    pub fn handle_trade_events(&mut self, events: &[TradeEvent]) {
        for ev in events {
            self.trades.push(ev.clone());
        }
    }
}
