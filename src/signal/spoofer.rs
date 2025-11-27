use crate::level2::OrderBook;
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use crate::trade::TradeStore;
use std::ops::Sub;

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

struct InnerDTO {
    added_qty: f32,
    executed_qty: f32,
    cancelled_qty: f32,
    average_qty: f32,
    side: Side,
    price: Price,
    period: Period,
    max_executed_rate: f32,
    min_cancel_rate: f32,
    min_score: f32,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(order_book: &'a OrderBook, trade_store: &'a TradeStore) -> Self {
        Self {
            order_book,
            trade_store,
        }
    }

    fn build_inner_dto(&self, dto: &FindSpoofersDTO, price: Price, side: Side) -> InnerDTO {
        let added_qty = self
            .order_book
            .get_side(side)
            .level_total_added(price, dto.period) as f32;
        let executed_qty = self
            .trade_store
            .level_executed_side(side, price, dto.period) as f32;
        let cancelled_qty: f32 = self
            .order_book
            .get_side(side)
            .level_total_outflow(price, dto.period)
            .saturating_sub(executed_qty as Quantity) as f32;
        let average_qty = self
            .order_book
            .get_side(side)
            .level_average_quantity(price, dto.period);

        InnerDTO {
            added_qty,
            executed_qty,
            cancelled_qty,
            average_qty,
            price,
            side,
            period: dto.period,
            max_executed_rate: dto.max_executed_rate,
            min_cancel_rate: dto.min_cancel_rate,
            min_score: dto.min_score,
        }
    }

    pub fn is_short_lived(&self, dto: &InnerDTO) -> bool {
        let (start_ts, end_ts) = dto.period;
        let duration = end_ts.saturating_sub(start_ts) as f32;

        if duration == 0
            || dto.average_qty == 0.0
            || dto.cancelled_qty == 0.0
            || dto.executed_qty == 0.0
        {
            return false;
        }
        let executed_lifetime = dto.average_qty / (dto.executed_qty / duration );
        let cancelled_lifetime = dto.average_qty / (dto.cancelled_qty / duration);

        cancelled_lifetime < executed_lifetime * 0.5
    }

    fn trade_price_intersect_price_level(&self, dto: &InnerDTO) -> bool {
        match dto.side {
            Side::Buy => {
                let edge = self.trade_store.min_price(dto.period);
                edge <= dto.price
            }
            Side::Sell => {
                let edge = self.trade_store.max_price(dto.period);
                edge >= dto.price
            }
        }
    }

    fn few_orders_were_executed(&self, dto: &InnerDTO) -> bool {
        todo!()
    }

    fn many_orders_were_cancelled(&self, dto: &InnerDTO) -> bool {
        todo!()
    }

    fn is_spoofer_here(&self, dto: &InnerDTO) -> bool {
        self.trade_price_intersect_price_level(dto)
            && self.few_orders_were_executed(dto)
            && self.many_orders_were_cancelled(dto)
    }

    pub fn execute(&self, dto: &FindSpoofersDTO) -> Result<Vec<SpooferDetected>, ()> {
        let mut result = Vec::new();

        for side in dto.sides.iter() {
            for price in self.order_book.get_side(*side).prices(dto.max_depth) {
                let inner_dto = self.build_inner_dto(dto, *price, *side);
                if self.is_spoofer_here(&inner_dto) {
                    for spike in self.order_book.get_side(*side).level_quantity_spikes(
                        *price,
                        dto.period,
                        dto.quantity_spike_threshold,
                    ) {
                        result.push(SpooferDetected {
                            side: *side,
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
