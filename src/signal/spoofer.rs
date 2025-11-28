use crate::level2::{OrderBook, QuantityStats};
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use crate::trade::{TradeStats, TradeStore};

pub struct SpooferDetected {
    pub side: Side,
    pub quantity: Quantity,
    pub price: Price,
    pub timestamp: TimestampMS,
}

pub struct FindSpoofers<'a> {
    order_book: &'a OrderBook,
    trade_store: &'a TradeStore,
    bid_stats: QuantityStats<'a>,
    ask_stats: QuantityStats<'a>,
    trade_stats: TradeStats<'a>,
}

pub struct FindSpoofersDTO {
    quantity_spike_rate: f32,
    cancelled_rate: f32,
    lifetime_rate: f32,
    executed_rate: f32,
    period: Period,
    max_depth: usize,
    sides: Vec<Side>,
}

struct InnerDTO {
    added_qty: f32,
    executed_qty: f32,
    cancelled_qty: f32,
    average_qty: f32,
    lifetime_rate: f32,
    executed_rate: f32,
    cancelled_rate: f32,
    side: Side,
    price: Price,
    period: Period,
}

impl<'a> FindSpoofers<'a> {
    pub fn new(order_book: &'a OrderBook, trade_store: &'a TradeStore) -> Self {
        Self {
            order_book,
            trade_store,
            bid_stats: QuantityStats::new(order_book.bids()),
            ask_stats: QuantityStats::new(order_book.asks()),
            trade_stats: TradeStats::new(trade_store),
        }
    }

    fn get_quantity_stats(&self, side: Side) -> &QuantityStats {
        match side {
            Side::Buy => &self.bid_stats,
            Side::Sell => &self.ask_stats,
        }
    }

    fn build_inner_dto(&self, dto: &FindSpoofersDTO, price: Price, side: Side) -> InnerDTO {
        let executed_qty = self
            .trade_stats
            .level_executed_side(side, price, dto.period) as f32;

        let qty_stats = self.get_quantity_stats(side);
        let average_qty = qty_stats.level_average_quantity(price, dto.period);
        let added_qty = qty_stats.level_total_added(price, dto.period) as f32;
        let cancelled_qty: f32 = qty_stats
            .level_total_outflow(price, dto.period)
            .saturating_sub(executed_qty as Quantity) as f32;

        InnerDTO {
            added_qty,
            executed_qty,
            cancelled_qty,
            average_qty,
            price,
            side,
            period: dto.period,
            executed_rate: dto.executed_rate,
            cancelled_rate: dto.cancelled_rate,
            lifetime_rate: dto.lifetime_rate,
        }
    }

    pub fn is_short_lived(&self, dto: &InnerDTO) -> bool {
        let (start_ts, end_ts) = dto.period;
        let duration = end_ts.saturating_sub(start_ts) as f32;

        if duration == 0.0
            || dto.average_qty == 0.0
            || dto.cancelled_qty == 0.0
            || dto.executed_qty == 0.0
        {
            return false;
        }
        let executed_lifetime = dto.average_qty / (dto.executed_qty / duration);
        let cancelled_lifetime = dto.average_qty / (dto.cancelled_qty / duration);

        cancelled_lifetime < executed_lifetime * dto.lifetime_rate
    }

    fn trade_price_intersect_price_level(&self, dto: &InnerDTO) -> bool {
        match dto.side {
            Side::Buy => {
                let edge = self.trade_stats.min_price(dto.period);
                edge <= dto.price
            }
            Side::Sell => {
                let edge = self.trade_stats.max_price(dto.period);
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
            for price in self.order_book.get_side(*side).best_prices(dto.max_depth) {
                let inner_dto = self.build_inner_dto(dto, *price, *side);
                if self.is_spoofer_here(&inner_dto) {
                    for spike in self.get_quantity_stats(*side).level_quantity_spikes(
                        *price,
                        dto.period,
                        dto.quantity_spike_rate,
                    ) {
                        result.push(SpooferDetected {
                            side: *side,
                            quantity: spike.quantity,
                            price: spike.price,
                            timestamp: spike.timestamp,
                        });
                    }
                }
            }
        }
        Ok(result)
    }
}
