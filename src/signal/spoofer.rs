use crate::level2::OrderBook;
use crate::shared::{Price, Quantity, Side, TimestampMS};
use crate::trade::TradeStore;

pub struct SpooferDetected {
    pub side: Side,
    pub quantity: Quantity,
    pub price: Price,
    pub score: u16,
    pub timestamp: TimestampMS,
}

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    trade_stats: &'a TradeStore,
}

pub struct FindSpoofersDTO {
    // todo!
}

impl<'a> FindSpoofers<'a> {
    pub fn new(order_book: &'a OrderBook, trade_stats: &'a TradeStore) -> Self {
        Self {
            order_book,
            trade_stats,
        }
    }

    pub fn execute(&self, dto: &FindSpoofersDTO) -> Result<Vec<SpooferDetected>, ()> {
        todo!()
    }
}
