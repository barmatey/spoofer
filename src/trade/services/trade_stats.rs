use crate::shared::{Period, Price, Quantity, Side};
use crate::trade::TradeStore;

pub struct TradeStats<'a> {
    trade_store: &'a TradeStore,
}

impl<'a> TradeStats<'a> {
    pub fn new(trade_store: &'a TradeStore) -> Self {
        Self { trade_store }
    }

    pub fn max_price(&self, period: Period) -> Price {
        self.trade_store.trades()
            .iter()
            .filter(|x| x.timestamp >= period.0 && x.timestamp < period.1)
            .map(|x| x.price)
            .max()
            .unwrap_or(0)
    }

    pub fn min_price(&self, period: Period) -> Price {
        self.trade_store.trades()
            .iter()
            .filter(|x| x.timestamp >= period.0 && x.timestamp < period.1)
            .map(|x| x.price)
            .min()
            .unwrap_or(Price::MAX)
    }

    pub fn level_executed(&self, price: Price, period: Period) -> Quantity {
        let (start_ts, end_ts) = period;

        self.trade_store.trades()
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

        self.trade_store.trades()
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
