use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::shared::{Period, Price, Quantity, Side};
use either::Either;
use std::collections::{BTreeSet, HashMap};

pub struct BookSide {
    ticks: HashMap<Price, Vec<LevelUpdated>>,
    sorted_prices: BTreeSet<Price>,
    side: Side,
    empty_ticks: Vec<LevelUpdated>,
}

impl BookSide {
    pub fn new(side: Side) -> Self {
        Self {
            ticks: HashMap::new(),
            sorted_prices: BTreeSet::new(),
            side,
            empty_ticks: vec![],
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
    
    pub fn level_ticks(&self, price: Price) -> &Vec<LevelUpdated> {
        &self.ticks.get(&price).unwrap_or(&self.empty_ticks)
    }
    
    pub fn best_prices(&self, depth: usize) -> impl Iterator<Item = &Price> {
        let iter = match self.side {
            Side::Buy => Either::Left(self.sorted_prices.iter().rev()),
            Side::Sell => Either::Right(self.sorted_prices.iter()),
        };
        iter.take(depth)
    }
    
    pub fn side(&self) -> &Side{
        &self.side
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
