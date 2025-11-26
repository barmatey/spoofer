use crate::level2::{BookStats, OrderBook};
use crate::spoofer::SpooferDetected;

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    book_stats: &'a BookStats,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(order_book: &'a OrderBook, book_stats: &'a BookStats) -> Self {
        Self {
            order_book,
            book_stats,
        }
    }

    pub fn execute(&self) -> Result<Vec<SpooferDetected>, ()> {
        todo!()
    }
}
