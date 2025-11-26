use crate::level2::events::LevelUpdated;
use crate::level2::traits::{BookSide, OrderBookFlowMetrics};
use crate::level2::Level2Error;
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use std::collections::{BTreeSet, HashMap};

struct BookSideRealization {
    ticks: HashMap<Price, Vec<LevelUpdated>>,
    sorted_prices: BTreeSet<Price>,
    side: Side,
}

impl BookSideRealization {
    pub fn new(side: Side) -> Self {
        Self {
            ticks: HashMap::new(),
            sorted_prices: BTreeSet::new(),
            side,
        }
    }

    pub(crate) fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        // Check event side
        if event.side != self.side {
            return Err(Level2Error::IncompatibleSide);
        }

        // Check event Timestamp
        if self
            .ticks
            .get(&event.price)
            .and_then(|v| v.last())
            .map_or(false, |last| event.timestamp < last.timestamp)
        {
            return Err(Level2Error::OutdatedEvent);
        }

        // Create price level if necessary
        if !self.ticks.contains_key(&event.price) {
            self.sorted_prices.insert(event.price);
        }

        self.ticks
            .entry(event.price)
            .or_insert_with(Vec::new)
            .push(event);

        Ok(())
    }

    fn total_change<F>(&self, price: Price, period: Period, compare: F) -> Quantity
    where
        F: Fn(Quantity, Quantity) -> Quantity,
    {
        let events = match self.ticks.get(&price) {
            Some(v) => v,
            None => return 0,
        };

        let (start_ts, end_ts) = period;

        let mut total: Quantity = 0;
        let mut last_qty: Option<Quantity> = None;

        for ev in events
            .iter()
            .filter(|ev| ev.timestamp >= start_ts && ev.timestamp < end_ts)
        {
            if let Some(prev) = last_qty {
                total += compare(ev.quantity, prev);
            }
            last_qty = Some(ev.quantity);
        }

        total
    }
}

impl BookSide for BookSideRealization {
    fn prices(&self) -> &BTreeSet<Price> {
        &self.sorted_prices
    }

    fn total_quantity(&self, depth: usize) -> Quantity {
        let iter: Box<dyn Iterator<Item = &Price>> = match self.side {
            Side::Buy => Box::new(self.sorted_prices.iter().rev()),
            Side::Sell => Box::new(self.sorted_prices.iter()),
        };

        iter.take(depth)
            .map(|price| {
                self.ticks
                    .get(price)
                    .and_then(|v| v.last())
                    .map_or(0, |ev| ev.quantity)
            })
            .sum()
    }
    fn level_quantity(&self, price: Price) -> Quantity {
        self.ticks
            .get(&price)
            .and_then(|v| v.last())
            .map_or(0, |last| last.quantity)
    }

    fn level_lifetime(&self, price: Price, period: Period) -> Option<TimestampMS> {
        let events = self.ticks.get(&price)?;
        let (start_ts, end_ts) = period;

        let first_nonzero = events
            .iter()
            .find(|ev| ev.timestamp >= start_ts && ev.timestamp < end_ts && ev.quantity > 0)
            .map(|ev| ev.timestamp)?;

        let last_nonzero = events
            .iter()
            .rev()
            .find(|ev| ev.timestamp >= start_ts && ev.timestamp < end_ts && ev.quantity > 0)
            .map(|ev| ev.timestamp)?;

        Some(last_nonzero.saturating_sub(first_nonzero))
    }

    fn level_average_quantity(&self, price: Price, period: Period) -> Quantity {
        let events = match self.ticks.get(&price) {
            Some(v) => v,
            None => return 0,
        };

        let (start_ts, end_ts) = period;
        let mut sum: Quantity = 0;
        let mut count: Quantity = 0;

        for ev in events.iter() {
            if ev.timestamp < start_ts {
                continue;
            }
            if ev.timestamp >= end_ts {
                break;
            }
            sum += ev.quantity;
            count += 1;
        }

        if count == 0 {
            0
        } else {
            sum / count
        }
    }

    fn level_total_added(&self, price: Price, period: Period) -> Quantity {
        self.total_change(price, period, |current, prev| {
            if current > prev {
                current - prev
            } else {
                0
            }
        })
    }

    fn level_total_cancelled(&self, price: Price, period: Period) -> Quantity {
        self.total_change(price, period, |current, prev| {
            if current < prev {
                prev - current
            } else {
                0
            }
        })
    }

    fn level_add_rate(&self, price: Price, period: Period) -> f32 {
        let total_added = self.level_total_added(price, period);

        let (start_ts, end_ts) = period;
        let duration = end_ts.saturating_sub(start_ts);

        if duration == 0 {
            0.0
        } else {
            total_added as f32 / duration as f32
        }
    }

    fn level_cancel_rate(&self, price: Price, period: Period) -> f32 {
        let total_cancelled = self.level_total_cancelled(price, period);

        let (start_ts, end_ts) = period;
        let duration = end_ts.saturating_sub(start_ts);

        if duration == 0 {
            0.0
        } else {
            total_cancelled as f32 / duration as f32
        }
    }

    fn level_volume_spike(&self, price: Price, period: Period, threshold: f32) -> bool {
        let average_volume_in_period = self.level_average_quantity(price, period) as f32;
        if average_volume_in_period == 0.0 {
            return false;
        }
        let total_added = self.level_total_added(price, period) as f32;
        total_added > average_volume_in_period * threshold
    }
}

pub struct OrderBook {
    bids: BookSideRealization,
    asks: BookSideRealization,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BookSideRealization::new(Side::Buy),
            asks: BookSideRealization::new(Side::Sell),
        }
    }
}

impl OrderBookFlowMetrics for OrderBook {
    fn bids(&self) -> &dyn BookSide {
        &self.bids
    }

    fn asks(&self) -> &dyn BookSide {
        &self.asks
    }

    fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        match event.side {
            Side::Buy => self.bids.update(event),
            Side::Sell => self.asks.update(event),
        }
    }

    fn bid_ask_pressure(&self, depth: usize) -> f32 {
        todo!()
    }
}
