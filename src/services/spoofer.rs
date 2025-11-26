use crate::domain::events::{LevelUpdated, Price, TimestampMS};
use crate::domain::{BookStats, OrderBook};

pub struct SpooferDetected {
    pub price: Price,
    pub score: u16,
    pub timestamp: TimestampMS,
}

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
