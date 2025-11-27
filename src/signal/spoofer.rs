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
    pub fn new(order_book: &'a OrderBook, trade_store: &'a TradeStore) -> Self {
        Self {
            order_book,
            trade_store,
        }
    }

    fn price_is_close_to_edge(&self, price: Price, side: Side, period: Period) -> bool {
        match side {
            Side::Buy => {
                let edge = self.trade_store.min_price(period);
                edge <= price
            }
            Side::Sell => {
                let edge = self.trade_store.max_price(period);
                edge >= price
            }
        }
    }

    pub fn execute(&self, dto: FindSpoofersDTO) -> Result<Vec<SpooferDetected>, ()> {
        let mut result = Vec::new();

        for side in dto.sides {
            let book = self.order_book.get_side(side);

            for price in book.prices(dto.max_depth) {
                let added_qty = book.level_total_added(*price, dto.period) as f32;
                let cancelled_qty = book.level_total_cancelled(*price, dto.period) as f32;
                let executed_qty =
                    self.trade_store
                        .level_executed_side(side, *price, dto.period) as f32;

                if cancelled_qty > added_qty * dto.min_cancel_rate // большая доля отмен
                    && executed_qty < added_qty * dto.max_executed_rate    // почти нет исполнения
                    && self.price_is_close_to_edge(*price, side, dto.period) // Цена доходила до заявки
                {
                    for spike in book.level_quantity_spikes(
                        *price,
                        dto.period,
                        dto.quantity_spike_threshold,
                    ) {
                        result.push(SpooferDetected {
                            side,
                            quantity: spike.quantity,
                            price: spike.price,
                            score: 0.0,
                            timestamp: spike.timestamp,
                        });
                    }
                }
            }
        }
        Ok(result)
    }
}
