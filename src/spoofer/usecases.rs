use crate::level2::{BookStats, OrderBook};
use crate::shared::{Price, TimestampMS};
use crate::spoofer::SpooferDetected;

pub struct FindSpoofer<'a> {
    order_book: &'a OrderBook,
    book_stats: &'a BookStats,
}

impl<'a> FindSpoofer<'a> {
    pub fn new(order_book: &'a OrderBook, book_stats: &'a BookStats) -> Self {
        Self {
            order_book,
            book_stats,
        }
    }

    pub fn execute(&self) -> Result<SpooferDetected, ()> {
        todo!()
    }
}
