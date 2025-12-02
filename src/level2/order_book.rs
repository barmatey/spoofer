use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::shared::errors::{check_exchange, check_side, check_ticker, check_timestamp};
use crate::shared::{Price, Side, TimestampMS};
use either::Either;
use std::collections::{BTreeSet, HashMap};

pub struct BookSide {
    ticks: HashMap<Price, Vec<LevelUpdated>>,
    sorted_prices: BTreeSet<Price>,
    side: Side,
    empty_ticks: Vec<LevelUpdated>,
    last_ts: TimestampMS,
}

impl BookSide {
    pub fn new(side: Side) -> Self {
        Self {
            ticks: HashMap::new(),
            sorted_prices: BTreeSet::new(),
            side,
            empty_ticks: vec![],
            last_ts: 0,
        }
    }

    pub(crate) fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        check_side(&self.side, &event.side)?;
        check_timestamp(self.last_ts, event.timestamp)?;

        if !self.ticks.contains_key(&event.price) {
            self.sorted_prices.insert(event.price);
        }

        self.last_ts = event.timestamp;

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
        iter.skip_while(|price| {
            self.ticks
                .get(price)
                .unwrap_or(&self.empty_ticks)
                .last()
                .map(|ev| ev.quantity)
                .unwrap_or(0)
                != 0
        })
        .take(depth)
    }

    pub fn side(&self) -> &Side {
        &self.side
    }
}

pub struct OrderBook {
    bids: BookSide,
    asks: BookSide,
    exchange: String,
    ticker: String,
}

impl OrderBook {
    pub fn new(exchange: &str, ticker: &str) -> Self {
        Self {
            bids: BookSide::new(Side::Buy),
            asks: BookSide::new(Side::Sell),
            exchange: exchange.to_string(),
            ticker: ticker.to_string(),
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
        check_exchange(&self.exchange, &event.exchange)?;
        check_ticker(&self.ticker, &event.ticker)?;

        match event.side {
            Side::Buy => self.bids.update(event),
            Side::Sell => self.asks.update(event),
        }
    }

    pub fn update_if_instrument_matches(mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        if self.ticker == event.ticker && self.exchange == event.exchange {
            match event.side {
                Side::Buy => self.bids.update(event)?,
                Side::Sell => self.asks.update(event)?,
            }
        }
        Ok(())
    }
}
