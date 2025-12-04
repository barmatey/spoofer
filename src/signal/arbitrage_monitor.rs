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
