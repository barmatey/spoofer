use crate::level2::OrderBook;
use crate::shared::Price;

#[derive(Debug, Clone)]
pub struct ArbitrageLeg {
    pub exchange: String,
    pub ticker: String,
    pub price: Price,
}

#[derive(Debug, Clone)]
pub struct ArbitrageSignal {
    pub buy: ArbitrageLeg,
    pub sell: ArbitrageLeg,
    pub profit_pct: f32,
    pub profit_abs: Option<f32>,
}

pub struct ArbitrageMonitor<'a> {
    book_a: &'a OrderBook,
    book_b: &'a OrderBook,
    min_profit_pct: f32, // минимальная прибыль в %
}

impl<'a> ArbitrageMonitor<'a> {
    pub fn new(book_a: &'a OrderBook, book_b: &'a OrderBook, min_profit_pct: f32) -> Self {
        Self {
            book_a,
            book_b,
            min_profit_pct,
        }
    }

    pub fn check_opportunity(&self) -> Option<ArbitrageSignal> {
        let bid_a = self.book_a.bids().best_price();
        let ask_a = self.book_a.asks().best_price();
        let bid_b = self.book_b.bids().best_price();
        let ask_b = self.book_b.asks().best_price();

        if let Some(sig) = self.check_pair(
            self.book_a,
            ask_a,
            self.book_b,
            bid_b,
        ) {
            return Some(sig);
        }

        if let Some(sig) = self.check_pair(
            self.book_b,
            ask_b,
            self.book_a,
            bid_a,
        ) {
            return Some(sig);
        }

        None
    }

    /// "buy on X → sell on Y"
    fn check_pair(
        &self,
        buy_book: &OrderBook,
        buy_price: Price,
        sell_book: &OrderBook,
        sell_price: Price,
    ) -> Option<ArbitrageSignal> {
        if sell_price <= buy_price {
            return None;
        }

        // нормализованный профит
        let profit_pct = (sell_price - buy_price) as f32 / buy_price as f32;

        if profit_pct < self.min_profit_pct {
            return None;
        }

        Some(ArbitrageSignal {
            buy: ArbitrageLeg {
                exchange: buy_book.exchange().to_string(),
                ticker: buy_book.ticker().to_string(),
                price: buy_price,
            },
            sell: ArbitrageLeg {
                exchange: sell_book.exchange().to_string(),
                ticker: sell_book.ticker().to_string(),
                price: sell_price,
            },
            profit_pct,
            profit_abs: Some((sell_price - buy_price) as f32),
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::level2::{LevelUpdated, OrderBook};
    use crate::shared::Side;

    fn ev(exchange: &str, ticker: &str, side: Side, price: Price, qty: u64) -> LevelUpdated {
        LevelUpdated {
            exchange: exchange.to_string(),
            ticker: ticker.to_string(),
            side,
            price,
            quantity: qty,
            timestamp: 0,
        }
    }

    #[test]
    fn test_no_opportunity() {
        let mut a = OrderBook::new("binance", "BTC/USDT", 10);
        let mut b = OrderBook::new("kraken", "BTC/USDT", 10);

        // A: bid=99, ask=100
        a.update(ev("binance", "BTC/USDT", Side::Buy, 99, 1)).unwrap();
        a.update(ev("binance", "BTC/USDT", Side::Sell, 100, 1)).unwrap();

        // B: bid=99, ask=100
        b.update(ev("kraken", "BTC/USDT", Side::Buy, 99, 1)).unwrap();
        b.update(ev("kraken", "BTC/USDT", Side::Sell, 100, 1)).unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.001);

        assert!(mon.check_opportunity().is_none());
    }

    #[test]
    fn test_simple_a_to_b() {
        let mut a = OrderBook::new("binance", "BTC/USDT", 10);
        let mut b = OrderBook::new("kraken", "BTC/USDT", 10);

        // A: buy at 100
        a.update(ev("binance", "BTC/USDT", Side::Sell, 100, 1)).unwrap();
        // B: sell at 103
        b.update(ev("kraken", "BTC/USDT", Side::Buy, 103, 1)).unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.0);

        let sig = mon.check_opportunity().expect("should detect arbitrage");

        assert_eq!(sig.buy.exchange, "binance");
        assert_eq!(sig.sell.exchange, "kraken");
        assert_eq!(sig.profit_abs, Some(3.0));
        assert!((sig.profit_pct - 0.03).abs() < 1e-6);
    }

    #[test]
    fn test_simple_b_to_a() {
        let mut a = OrderBook::new("binance", "BTC/USDT", 10);
        let mut b = OrderBook::new("kraken", "BTC/USDT", 10);

        // B: ask = 100
        b.update(ev("kraken", "BTC/USDT", Side::Sell, 100, 1)).unwrap();
        // A: bid = 105
        a.update(ev("binance", "BTC/USDT", Side::Buy, 105, 1)).unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.0);

        let sig = mon.check_opportunity().expect("should detect arbitrage");
        assert_eq!(sig.buy.exchange, "kraken");
        assert_eq!(sig.sell.exchange, "binance");
        assert_eq!(sig.profit_abs, Some(5.0));
        assert!((sig.profit_pct - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_profit_below_threshold() {
        let mut a = OrderBook::new("binance", "BTC/USDT", 10);
        let mut b = OrderBook::new("kraken", "BTC/USDT", 10);

        // A: ask = 100
        a.update(ev("binance", "BTC/USDT", Side::Sell, 100, 1)).unwrap();
        // B: bid = 100.05
        b.update(ev("kraken", "BTC/USDT", Side::Buy, 10005, 1)).unwrap(); // <--- если Price = 10000 => 100.00

        // threshold = 0.001 = 0.1%
        let mon = ArbitrageMonitor::new(&a, &b, 0.001);

        assert!(mon.check_opportunity().is_none());
    }

    #[test]
    fn test_uses_real_best_prices() {
        let mut a = OrderBook::new("binance", "BTC/USDT", 10);
        let mut b = OrderBook::new("kraken", "BTC/USDT", 10);

        // An asks: 100, 101, 102 → best = 100
        a.update(ev("binance", "BTC/USDT", Side::Sell, 102, 1)).unwrap();
        a.update(ev("binance", "BTC/USDT", Side::Sell, 101, 1)).unwrap();
        a.update(ev("binance", "BTC/USDT", Side::Sell, 100, 1)).unwrap();

        // B bids: 99, 103, 101 → best = 103
        b.update(ev("kraken", "BTC/USDT", Side::Buy, 99, 1)).unwrap();
        b.update(ev("kraken", "BTC/USDT", Side::Buy, 103, 1)).unwrap();
        b.update(ev("kraken", "BTC/USDT", Side::Buy, 101, 1)).unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.0);

        let sig = mon.check_opportunity().unwrap();

        assert_eq!(sig.buy.price, 100);
        assert_eq!(sig.sell.price, 103);
    }
}
