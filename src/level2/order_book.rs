use crate::level2::events::LevelUpdated;
use crate::level2::traits::OrderBookFlowMetrics;
use crate::level2::Level2Error;
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use std::collections::HashMap;

struct BookSide {
    ticks: HashMap<Price, Vec<LevelUpdated>>,
    side: Side,
}

impl BookSide {
    pub fn new(side: Side) -> Self {
        Self {
            ticks: HashMap::new(),
            side,
        }
    }

    pub fn handle_level_updated(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        // Check side
        if event.side != self.side {
            return Err(Level2Error::IncompatibleSide);
        }

        // Check timestamp
        if self
            .ticks
            .get(&event.price)
            .and_then(|v| v.last())
            .map_or(false, |last| event.timestamp < last.timestamp)
        {
            return Err(Level2Error::OutdatedEvent);
        }

        // Push event
        self.ticks
            .entry(event.price)
            .or_insert_with(Vec::new)
            .push(event);

        Ok(())
    }

    pub fn current_quantity(&self, price: Price) -> Quantity {
        self.ticks
            .get(&price)
            .and_then(|v| v.last())
            .map_or(0, |last| last.quantity)
    }

    pub fn avg_quantity(&self, price: Price, period: Period) -> Quantity {
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

    pub fn total_added(&self, price: Price, period: Period) -> Quantity {
        self.total_change(price, period, |current, prev| {
            if current > prev {
                current - prev
            } else {
                0
            }
        })
    }

    pub fn total_cancelled(&self, price: Price, period: Period) -> Quantity {
        self.total_change(price, period, |current, prev| {
            if current < prev {
                prev - current
            } else {
                0
            }
        })
    }

    pub fn add_rate(&self, price: Price, period: Period) -> f32 {
        let total_added = self.total_added(price, period);

        let (start_ts, end_ts) = period;
        let duration = end_ts.saturating_sub(start_ts);

        if duration == 0 {
            0.0
        } else {
            total_added as f32 / duration as f32
        }
    }

    pub fn cancel_rate(&self, price: Price, period: Period) -> f32 {
        let total_cancelled = self.total_cancelled(price, period);

        let (start_ts, end_ts) = period;
        let duration = end_ts.saturating_sub(start_ts);

        if duration == 0 {
            0.0
        } else {
            total_cancelled as f32 / duration as f32
        }
    }

    pub fn level_lifetime(&self, price: Price, period: Period) -> Option<TimestampMS> {
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

    pub fn is_volume_spike(&self, price: Price, period: Period, threshold: f32) -> bool {
        let average_volume_in_period = self.avg_quantity(price, period) as f32;
        if average_volume_in_period == 0.0 {
            return false;
        }
        let total_added = self.total_added(price, period) as f32;
        total_added > average_volume_in_period * threshold
    }
}

pub struct OrderBook {
    bids: BookSide,
    asks: BookSide,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BookSide::new(Side::Buy),
            asks: BookSide::new(Side::Sell),
        }
    }
}

impl OrderBookFlowMetrics for OrderBook {
    fn handle_level_updated(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        match event.side {
            Side::Buy => self.bids.handle_level_updated(event),
            Side::Sell => self.asks.handle_level_updated(event),
        }
    }

    fn current_quantity(&self, price: Price, side: Side) -> Quantity {
        match side {
            Side::Buy => self.bids.current_quantity(price),
            Side::Sell => self.asks.current_quantity(price),
        }
    }

    fn book_pressure(&self, side: Side, depth: usize) -> f32 {
        todo!()
    }

    fn level_lifetime(&self, price: Price, side: Side, period: Period) -> Option<TimestampMS> {
        match side {
            Side::Buy => self.bids.level_lifetime(price, period),
            Side::Sell => self.asks.level_lifetime(price, period),
        }
    }

    fn avg_quantity(&self, price: Price, side: Side, period: Period) -> Quantity {
        match side {
            Side::Buy => self.bids.avg_quantity(price, period),
            Side::Sell => self.asks.avg_quantity(price, period),
        }
    }

    fn total_added(&self, price: Price, side: Side, period: Period) -> Quantity {
        match side {
            Side::Buy => self.bids.total_added(price, period),
            Side::Sell => self.asks.total_added(price, period),
        }
    }

    fn total_cancelled(&self, price: Price, side: Side, period: Period) -> Quantity {
        match side {
            Side::Buy => self.bids.total_cancelled(price, period),
            Side::Sell => self.asks.total_cancelled(price, period),
        }
    }

    fn add_rate(&self, price: Price, side: Side, period: Period) -> f32 {
        match side {
            Side::Buy => self.bids.add_rate(price, period),
            Side::Sell => self.asks.add_rate(price, period),
        }
    }

    fn cancel_rate(&self, price: Price, side: Side, period: Period) -> f32 {
        match side {
            Side::Buy => self.bids.cancel_rate(price, period),
            Side::Sell => self.asks.cancel_rate(price, period),
        }
    }

    fn is_volume_spike(&self, price: Price, side: Side, period: Period, threshold: f32) -> bool {
        match side {
            Side::Buy => self.bids.is_volume_spike(price, period, threshold),
            Side::Sell => self.asks.is_volume_spike(price, period, threshold),
        }
    }
}
