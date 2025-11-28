use crate::level2::order_book::BookSide;
use crate::level2::LevelUpdated;
use crate::shared::{Period, Price, Quantity, Side};

pub struct QuantityStatService<'a> {
    side: &'a BookSide,
}

impl<'a> QuantityStatService<'a> {
    pub fn total_quantity(&self, depth: usize) -> Quantity {
        self.side
            .prices(depth)
            .filter_map(|price| self.side.ticks(*price).last())
            .map(|tick| tick.quantity)
            .sum()
    }

    pub fn level_quantity(&self, price: Price) -> Quantity {
        self.side.ticks(price).last().map_or(0, |ev| ev.quantity)
    }

    pub fn level_average_quantity(&self, price: Price, period: Period) -> f32 {
        let (start_ts, end_ts) = period;

        let sum_count = self
            .side
            .ticks(price)
            .iter()
            .rev()
            .skip_while(|ev| ev.timestamp > end_ts)
            .take_while(|ev| ev.timestamp >= start_ts)
            .fold((0f32, 0u16), |(sum, count), ev| {
                (sum + ev.quantity as f32, count + 1)
            });

        let (sum, count) = sum_count;

        if count == 0 { 0.0 } else { sum / count as f32 }
    }


    pub fn level_total_added(&self, price: Price, period: Period) -> Quantity {
        self.total_change(price, period, |current, prev| {
            if current > prev {
                current - prev
            } else {
                0
            }
        })
    }

    pub fn level_total_outflow(&self, price: Price, period: Period) -> Quantity {
        self.total_change(price, period, |current, prev| {
            if current < prev {
                prev - current
            } else {
                0
            }
        })
    }

    pub fn level_quantity_spikes(
        &self,
        price: Price,
        period: Period,
        threshold: f32,
    ) -> impl Iterator<Item = &LevelUpdated> {
        let avg = self.level_average_quantity(price, period);

        self.ticks
            .get(&price)
            .into_iter()
            .flat_map(|v| v.iter())
            .filter(move |x| (x.quantity as f32) > (avg * threshold))
    }
}
