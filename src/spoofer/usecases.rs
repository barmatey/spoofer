use crate::level2::{OrderBook};
use crate::spoofer::SpooferDetected;
use crate::trade::TradeStats;

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    trade_stats: &'a TradeStats,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(
        order_book: &'a OrderBook,
        trade_stats: &'a TradeStats,
    ) -> Self {
        Self {
            order_book,
            trade_stats,
        }
    }

    pub fn execute(&self) -> Result<Vec<SpooferDetected>, ()> {
        todo!()
    }
}
