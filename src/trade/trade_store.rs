use crate::shared::{Period, Price, Quantity, Side};
use crate::trade::errors::TradeError;
use crate::trade::traits::TradeStore;
use crate::trade::TradeEvent;

pub struct TradeStoreRealisation {
    trades: Vec<TradeEvent>,
}

impl TradeStoreRealisation {
    pub fn new() -> Self {
        Self { trades: Vec::new() }
    }
}

impl TradeStore for TradeStoreRealisation {
    fn handle_trade(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        if let Some(last) = self.trades.last() {
            if trade.timestamp < last.timestamp {
                return Err(TradeError::OutdatedEvent);
            }
        }
        self.trades.push(trade);
        Ok(())
    }

    fn level_executed(&self, price: Price, period: Period) -> Quantity {
        let (start_ts, end_ts) = period;

        self.trades
            .iter()
            .filter(|tr| tr.price == price && tr.timestamp >= start_ts && tr.timestamp < end_ts)
            .map(|tr| tr.quantity)
            .sum()
    }

    fn level_executed_bid(&self, price: Price, period: Period) -> Quantity {
        let (start_ts, end_ts) = period;

        self.trades
            .iter()
            .filter(|tr| {
                tr.price == price
                    && tr.taker == Side::Sell
                    && tr.timestamp >= start_ts
                    && tr.timestamp < end_ts
            })
            .map(|tr| tr.quantity)
            .sum()
    }

    fn level_executed_ask(&self, price: Price, period: Period) -> Quantity {
        let (start_ts, end_ts) = period;

        self.trades
            .iter()
            .filter(|tr| {
                tr.price == price
                    && tr.taker == Side::Buy
                    && tr.timestamp >= start_ts
                    && tr.timestamp < end_ts
            })
            .map(|tr| tr.quantity)
            .sum()
    }
    fn level_executed_side(&self, taker: Side, price: Price, period: Period) -> Quantity {
        todo!()
    }
}
