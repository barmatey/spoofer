use crate::level2::OrderBook;
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use crate::trade::{TradeStore};

pub struct SpooferDetected {
    pub side: Side,
    pub quantity: Quantity,
    pub price: Price,
    pub score: f32,
    pub timestamp: TimestampMS,
}

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    trade_store: &'a TradeStore,
}

pub struct FindSpoofersDTO {
    min_score: f32,
    quantity_spike_threshold: f32,
    min_cancel_rate: f32,
    max_executed_rate: f32,
    period: Period,
    max_depth: usize,
    sides: Vec<Side>,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(
        order_book: &'a OrderBook,
        trade_store: &'a TradeStore,
    ) -> Self {
        Self {
            order_book,
            trade_store,
        }
    }

    pub fn execute(&self, dto: FindSpoofersDTO) -> Result<Vec<SpooferDetected>, ()> {
        let mut result = Vec::new();
        for side in dto.sides {
            let orders = self.order_book.get_side(side);

            for price in orders.prices() {
                let added_qty = orders.level_total_added(*price, dto.period);
                let cancelled_qty = orders.level_total_cancelled(*price, dto.period);
                let executed_qty = self
                    .trade_store
                    .level_executed_side(side, *price, dto.period);
                let quantity_spikes =
                    orders.level_quantity_spikes(*price, dto.period, dto.quantity_spike_threshold);
            }
        }
        Ok(result)
    }
}
