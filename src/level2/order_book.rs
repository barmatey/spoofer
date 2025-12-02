use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::level2::Level2Error::IncompatibleExchange;
use crate::shared::{Price, Side};
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

    fn check_event_side(&self, event: &LevelUpdated) -> Result<(), Level2Error>{
        if event.side != self.side {
            return Err(Level2Error::IncompatibleSide(format!(
                "book_side != event.side, {:?} != {:?}",
                self.side, event.side
            )));
        }
        Ok(())
    }

    fn check_event_timestamp(&self, event:&LevelUpdated) -> Result<(), Level2Error>{
        if self
            .ticks
            .get(&event.price)
            .and_then(|v| v.last())
            .map_or(false, |last| event.timestamp < last.timestamp)
        {
            return Err(Level2Error::OutdatedEvent(
                "You are trying to add an event that earliest last one".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        self.check_event_side(&event)?;
        self.check_event_timestamp(&event)?;

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

    fn check_exchange(&self, event: &LevelUpdated) -> Result<(), Level2Error> {
        if event.exchange != self.exchange {
            Err(IncompatibleExchange(format!(
                "OrderBook.exchange != event.exchange. {} != {}",
                self.exchange, event.exchange
            )))?;
        }
        Ok(())
    }

    fn check_ticker(&self, event: &LevelUpdated) -> Result<(), Level2Error> {
        if event.ticker != self.ticker {
            Err(IncompatibleExchange(format!(
                "OrderBook.ticker != event.ticker. {} != {}",
                self.ticker, event.ticker
            )))?;
        }
        Ok(())
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
        self.check_exchange(&event)?;
        self.check_ticker(&event)?;

        match event.side {
            Side::Buy => self.bids.update(event),
            Side::Sell => self.asks.update(event),
        }
    }

    pub fn update_or_miss(&mut self, event: LevelUpdated){

    }
}
