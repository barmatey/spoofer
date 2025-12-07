use std::sync::Arc;
use crate::level2::book_side::BookSide;
use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::shared::errors::{check_exchange, check_ticker};
use crate::shared::{Exchange, Side};

pub struct OrderBook {
    bids: BookSide,
    asks: BookSide,
    exchange: Exchange,
    ticker: Arc<String>,
}

impl OrderBook {
    pub fn new(exchange: Exchange, ticker: &str, max_depth: usize) -> Self {
        Self {
            bids: BookSide::new(Side::Buy, max_depth),
            asks: BookSide::new(Side::Sell, max_depth),
            exchange,
            ticker: Arc::new(ticker.to_string()),
        }
    }

    pub fn exchange(&self) -> &Exchange{
        &self.exchange
    }

    pub fn ticker(&self) -> &Arc<String>{
        &self.ticker
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

    pub fn update(&mut self, event: &LevelUpdated) -> Result<(), Level2Error> {
        check_exchange(&self.exchange, &event.exchange)?;
        check_ticker(&self.ticker, &event.ticker)?;
        match event.side {
            Side::Buy => self.bids.update(event),
            Side::Sell => self.asks.update(event),
        }
    }

    pub fn update_if_instrument_matches(&mut self, event: &LevelUpdated) -> Result<(), Level2Error> {
        if self.ticker == event.ticker && self.exchange == event.exchange {
            match event.side {
                Side::Buy => self.bids.update(event)?,
                Side::Sell => self.asks.update(event)?,
            }
        }
        Ok(())
    }

    pub fn update_or_miss(&mut self, event: &LevelUpdated) {
        if self.ticker == event.ticker && self.exchange == event.exchange {
            match event.side {
                Side::Buy => self.bids.update_or_miss(event),
                Side::Sell => self.asks.update_or_miss(event),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level2::LevelUpdated;
    use crate::shared::Side;

    fn event(exchange: Exchange, ticker: &str, side: Side, price: u64, qty: u64) -> LevelUpdated {
        LevelUpdated {
            exchange,
            ticker: Arc::new(ticker.to_string()),
            side,
            price,
            quantity: qty,
            timestamp: 0,
        }
    }

    #[test]
    fn test_insert_bids_and_asks() {
        let mut ob = OrderBook::new(Exchange::Binance, "BTCUSDT", 5);
        ob.update(&event(Exchange::Binance, "BTCUSDT", Side::Buy, 100, 10)).unwrap();
        ob.update(&event(Exchange::Binance, "BTCUSDT", Side::Sell, 200, 5)).unwrap();
        assert_eq!(ob.bids().best_price(), 100);
        assert_eq!(ob.asks().best_price(), 200);
    }

    #[test]
    fn test_update_existing_levels() {
        let mut ob = OrderBook::new(Exchange::Binance, "BTCUSDT", 5);
        ob.update(&event(Exchange::Binance, "BTCUSDT", Side::Buy, 100, 10)).unwrap();
        ob.update(&event(Exchange::Binance, "BTCUSDT", Side::Buy, 100, 20)).unwrap();
        ob.update(&event(Exchange::Binance, "BTCUSDT", Side::Sell, 200, 5)).unwrap();
        ob.update(&event(Exchange::Binance, "BTCUSDT", Side::Sell, 200, 0)).unwrap();
        assert_eq!(ob.asks().best_price(), 0);
    }

    #[test]
    fn test_update_if_instrument_matches() {
        let mut ob = OrderBook::new(Exchange::Binance, "BTCUSDT", 5);

        // Совпадает инструмент
        ob.update_if_instrument_matches(&event(Exchange::Binance, "BTCUSDT", Side::Buy, 100, 10)).unwrap();
        assert_eq!(ob.bids().best_price(), 100);

        // Несовпадает тикер
        ob.update_if_instrument_matches(&event(Exchange::Binance, "ETHUSDT", Side::Buy, 150, 5)).unwrap();
        assert_eq!(ob.bids().best_price(), 100);

        // Несовпадает биржа
        ob.update_if_instrument_matches(&event(Exchange::Kraken, "BTCUSDT", Side::Buy, 200, 5)).unwrap();
        assert_eq!(ob.bids().best_price(), 100);
    }

    #[test]
    fn test_update_or_miss() {
        let mut ob = OrderBook::new(Exchange::Binance, "BTCUSDT", 5);

        // Совпадает инструмент
        ob.update_or_miss(&&event(Exchange::Binance, "BTCUSDT", Side::Buy, 100, 10));
        assert_eq!(ob.bids().best_price(), 100);

        // Несовпадает инструмент, ничего не должно происходить
        ob.update_or_miss(&&event(Exchange::Kraken, "BTCUSDT", Side::Buy, 200, 5));
        assert_eq!(ob.bids().best_price(), 100);
    }

    #[test]
    fn test_get_side() {
        let ob = OrderBook::new(Exchange::Binance, "BTCUSDT", 5);
        assert!(matches!(ob.get_side(Side::Buy).side(), &Side::Buy));
        assert!(matches!(ob.get_side(Side::Sell).side(), &Side::Sell));
    }

    #[test]
    fn test_exchange_and_ticker_check() {
        let mut ob = OrderBook::new(Exchange::Binance, "BTCUSDT", 5);

        // Неправильная биржа
        let err = ob.update(&event(Exchange::Kraken, "BTCUSDT", Side::Buy, 100, 10));
        assert!(err.is_err());

        // Неправиль тикер
        let err = ob.update(&event(Exchange::Binance, "ETHUSDT", Side::Buy, 100, 10));
        assert!(err.is_err());
    }
}
