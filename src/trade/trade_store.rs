use crate::shared::{Period, Price, Quantity, Side};
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

    pub fn level_executed(&self, price: Price, period: Period) -> Quantity {
        let (start_ts, end_ts) = period;

        self.trades
            .iter()
            .filter(|tr| tr.price == price && tr.timestamp >= start_ts && tr.timestamp < end_ts)
            .map(|tr| tr.quantity)
            .sum()
    }

    pub fn level_executed_bid(&self, price: Price, period: Period) -> Quantity {
        self.level_executed_side(Side::Buy, price, period)

    }

    pub fn level_executed_ask(&self, price: Price, period: Period) -> Quantity {
        self.level_executed_side(Side::Sell, price, period)
    }
    pub fn level_executed_side(&self, side: Side, price: Price, period: Period) -> Quantity {
        let (start_ts, end_ts) = period;

        self.trades
            .iter()
            .filter(|tr| {
                tr.price == price
                    && tr.market_maker == side
                    && tr.timestamp >= start_ts
                    && tr.timestamp < end_ts
            })
            .map(|tr| tr.quantity)
            .sum() 
    }
}
