use crate::level2::OrderBook;
use crate::shared::utils::now_timestamp;
use crate::shared::{Price};
use std::sync::Arc;
use crate::signal::arbitrage_monitor::ArbitrageSignal;
use crate::signal::arbitrage_monitor::signal::ArbitrageLeg;

pub struct ArbitrageMonitor<'a> {
    book_a: &'a OrderBook,
    book_b: &'a OrderBook,
    min_profit: f32, // минимальная прибыль в %
}

impl<'a> ArbitrageMonitor<'a> {
    pub fn new(book_a: &'a OrderBook, book_b: &'a OrderBook, min_profit: f32) -> Self {
        Self {
            book_a,
            book_b,
            min_profit,
        }
    }

    pub fn execute(&self) -> Option<ArbitrageSignal> {
        if self.book_a.bids().is_empty()
            || self.book_a.asks().is_empty()
            || self.book_b.bids().is_empty()
            || self.book_b.asks().is_empty()
        {
            return None;
        }

        let bid_a = self.book_a.bids().best_price();
        let ask_a = self.book_a.asks().best_price();
        let bid_b = self.book_b.bids().best_price();
        let ask_b = self.book_b.asks().best_price();

        if bid_a == None || bid_b == None || ask_a == None || ask_b == None {
            return None;
        }

        if let Some(sig) = self.check_pair(self.book_a, ask_a.unwrap(), self.book_b, bid_b.unwrap())
        {
            return Some(sig);
        }

        if let Some(sig) = self.check_pair(self.book_b, ask_b.unwrap(), self.book_a, bid_a.unwrap())
        {
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

        if profit_pct < self.min_profit {
            return None;
        }

        Some(ArbitrageSignal {
            buy: ArbitrageLeg {
                exchange: buy_book.exchange().clone(),
                ticker: Arc::clone(buy_book.ticker()),
                price: buy_price,
            },
            sell: ArbitrageLeg {
                exchange: sell_book.exchange().clone(),
                ticker: Arc::clone(sell_book.ticker()),
                price: sell_price,
            },
            profit_pct,
            profit_abs: (sell_price - buy_price) as f32,
            timestamp: now_timestamp(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level2::{LevelUpdated, OrderBook};
    use crate::shared::utils::now_timestamp_ns;
    use crate::shared::{Exchange, Side};

    fn ev(exchange: Exchange, ticker: &str, side: Side, price: Price, qty: u64) -> LevelUpdated {
        LevelUpdated {
            exchange,
            ticker: Arc::new(ticker.to_string()),
            side,
            price,
            quantity: qty,
            timestamp: 0,
            received: now_timestamp_ns(),
        }
    }

    #[test]
    fn test_no_opportunity() {
        let mut a = OrderBook::new(Exchange::Binance, "BTC/USDT", 10);
        let mut b = OrderBook::new(Exchange::Kraken, "BTC/USDT", 10);

        // A: bid=99, ask=100
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Buy, 99, 1))
            .unwrap();
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 100, 1))
            .unwrap();

        // B: bid=99, ask=100
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 99, 1))
            .unwrap();
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Sell, 100, 1))
            .unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.001);

        assert!(mon.execute().is_none());
    }

    #[test]
    fn test_simple_a_to_b() {
        let mut a = OrderBook::new(Exchange::Binance, "BTC/USDT", 10);
        let mut b = OrderBook::new(Exchange::Kraken, "BTC/USDT", 10);

        // Fake levels
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Buy, 99, 1))
            .unwrap();
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Sell, 104, 2))
            .unwrap();

        // A: buy at 100
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 100, 1))
            .unwrap();
        // B: sell at 103
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 103, 1))
            .unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.0);

        let sig = mon.execute().expect("should detect arbitrage");

        assert_eq!(sig.buy.exchange, Exchange::Binance);
        assert_eq!(sig.sell.exchange, Exchange::Kraken);
        assert_eq!(sig.profit_abs,3.0);
        assert!((sig.profit_pct - 0.03).abs() < 1e-6);
    }

    #[test]
    fn test_simple_b_to_a() {
        let mut a = OrderBook::new(Exchange::Binance, "BTC/USDT", 10);
        let mut b = OrderBook::new(Exchange::Kraken, "BTC/USDT", 10);

        // Fake levels

        // B: ask = 100
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 99, 1))
            .unwrap();
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Sell, 100, 1))
            .unwrap();

        // A: bid = 105
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Buy, 105, 1))
            .unwrap();
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 111, 2))
            .unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.0);

        let sig = mon.execute().expect("should detect arbitrage");
        assert_eq!(sig.buy.exchange, Exchange::Kraken);
        assert_eq!(sig.sell.exchange, Exchange::Binance);
        assert_eq!(sig.profit_abs, 5.0);
        assert!((sig.profit_pct - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_profit_below_threshold() {
        let mut a = OrderBook::new(Exchange::Binance, "BTC/USDT", 10);
        let mut b = OrderBook::new(Exchange::Kraken, "BTC/USDT", 10);

        // A: ask = 100
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 10_000, 1))
            .unwrap();
        // B: bid = 100.05
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 10_005, 1))
            .unwrap(); // <--- если Price = 10000 => 100.00

        // threshold = 0.001 = 0.1%
        let mon = ArbitrageMonitor::new(&a, &b, 0.001);

        assert!(mon.execute().is_none());
    }

    #[test]
    fn test_uses_real_best_prices() {
        let mut a = OrderBook::new(Exchange::Binance, "BTC/USDT", 10);
        let mut b = OrderBook::new(Exchange::Kraken, "BTC/USDT", 10);

        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Buy, 99, 1))
            .unwrap();
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Sell, 103, 1))
            .unwrap();

        // An asks: 100, 101, 102 → best = 100
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 102, 1))
            .unwrap();
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 101, 1))
            .unwrap();
        a.update(&ev(Exchange::Binance, "BTC/USDT", Side::Sell, 100, 1))
            .unwrap();

        // B bids: 99, 103, 101 → best = 103
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 99, 1))
            .unwrap();
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 103, 1))
            .unwrap();
        b.update(&ev(Exchange::Kraken, "BTC/USDT", Side::Buy, 101, 1))
            .unwrap();

        let mon = ArbitrageMonitor::new(&a, &b, 0.0);

        let sig = mon.execute().unwrap();

        assert_eq!(sig.buy.price, 100);
        assert_eq!(sig.sell.price, 103);
    }

    #[test]
    fn test_arbitrage_monitor_bug_infinite_profit() {
        // Создаём 2 книги
        let book_a = OrderBook::new(Exchange::Binance, "btc/usdt", 10);
        let mut book_b = OrderBook::new(Exchange::Kraken, "btc/usdt", 10);

        // book_b имеет только bid-уровень
        book_b
            .update(&ev(Exchange::Kraken, "btc/usdt", Side::Buy, 100, 1))
            .unwrap();

        // book_a остаётся с default ask = 0 (пустая книга)
        // что создаёт неверную ситуацию best_price == 0

        // Создаём монитор
        let mon = ArbitrageMonitor::new(&book_a, &book_b, 0.05);

        // Выполняем арбитраж
        let sig = mon.execute();
        assert!(sig.is_none());
    }
}
