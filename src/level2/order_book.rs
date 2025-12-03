use crate::level2::book_side::BookSide;
use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::shared::errors::{check_exchange, check_ticker};
use crate::shared::Side;

pub struct OrderBook {
    bids: BookSide,
    asks: BookSide,
    exchange: String,
    ticker: String,
}

impl OrderBook {
    pub fn new(exchange: &str, ticker: &str, max_depth: usize, max_ticks: usize) -> Self {
        Self {
            bids: BookSide::new(Side::Buy, max_depth),
            asks: BookSide::new(Side::Sell, max_depth),
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

    pub fn update_if_instrument_matches(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        if self.ticker == event.ticker && self.exchange == event.exchange {
            match event.side {
                Side::Buy => self.bids.update(event)?,
                Side::Sell => self.asks.update(event)?,
            }
        }
        Ok(())
    }

    pub fn update_or_miss(&mut self, event: LevelUpdated) {
        if self.ticker == event.ticker && self.exchange == event.exchange {
            match event.side {
                Side::Buy => self.bids.update_or_miss(event),
                Side::Sell => self.asks.update_or_miss(event),
            }
        }
    }
}
