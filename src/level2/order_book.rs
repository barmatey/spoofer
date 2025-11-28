use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::shared::{Period, Price, Quantity, Side};
use either::Either;
use std::collections::{BTreeSet, HashMap};

pub struct BookSide {
    ticks: HashMap<Price, Vec<LevelUpdated>>,
    sorted_prices: BTreeSet<Price>,
    side: Side,
}

impl BookSide {
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

    pub fn prices(&self, depth: usize) -> impl Iterator<Item = &Price> {
        let iter = match self.side {
            Side::Buy => Either::Left(self.sorted_prices.iter().rev()),
            Side::Sell => Either::Right(self.sorted_prices.iter()),
        };
        iter.take(depth)
    }

    pub fn total_quantity(&self, depth: usize) -> Quantity {
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
    pub fn level_quantity(&self, price: Price) -> Quantity {
        self.ticks
            .get(&price)
            .and_then(|v| v.last())
            .map_or(0, |last| last.quantity)
    }

    pub fn level_average_quantity(&self, price: Price, period: Period) -> f32 {
        let events = match self.ticks.get(&price) {
            Some(v) => v,
            None => return 0.,
        };

        let (start_ts, end_ts) = period;
        let mut sum: f32 = 0.;
        let mut count: u16 = 0;

        for ev in events.iter() {
            if ev.timestamp < start_ts {
                continue;
            }
            if ev.timestamp >= end_ts {
                break;
            }
            sum += ev.quantity as f32;
            count += 1;
        }

        if count == 0 {
            0.
        } else {
            sum / count as f32
        }
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

    pub fn bids(&self) -> &BookSide {
        &self.bids
    }

    pub fn asks(&self) -> &BookSide {
        &self.asks
    }

    pub fn get_side(&self, side: Side) -> &BookSide {
        match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        }
    }

    pub fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        match event.side {
            Side::Buy => self.bids.update(event),
            Side::Sell => self.asks.update(event),
        }
    }
}
