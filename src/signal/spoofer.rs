use crate::level2::OrderBook;
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use crate::trade::TradeStore;

pub struct SpooferDetected {
    pub side: Side,
    pub quantity: Quantity,
    pub price: Price,
    pub score: f32,
    pub timestamp: TimestampMS,
}

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    trade_stats: &'a TradeStore,
}

pub struct FindSpoofersDTO {
    min_score: f32,
    min_quantity_spike: f32,
    min_cancel_rate: f32,
    max_executed_rate: f32,
    average_period: Period,
    search_period: Period,
    max_depth: usize,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(order_book: &'a OrderBook, trade_stats: &'a TradeStore) -> Self {
        Self {
            order_book,
            trade_stats,
        }
    }

    pub fn execute(&self, dto: FindSpoofersDTO) -> Result<Vec<SpooferDetected>, ()> {
        todo!()
    }
}
