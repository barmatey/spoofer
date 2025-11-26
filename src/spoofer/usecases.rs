use crate::level2::{BookStats, OrderBook};
use crate::spoofer::SpooferDetected;
use crate::trade::TradeStats;

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    book_stats: &'a BookStats,
    trade_stats: &'a TradeStats,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(
        order_book: &'a OrderBook,
        book_stats: &'a BookStats,
        trade_stats: &'a TradeStats,
    ) -> Self {
        Self {
            order_book,
            book_stats,
            trade_stats,
        }
    }

    pub fn execute(&self) -> Result<Vec<SpooferDetected>, ()> {
        todo!()
    }
}
