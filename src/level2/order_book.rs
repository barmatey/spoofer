use crate::level2::events::LevelUpdated;
use crate::level2::Level2Error;
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};
use std::collections::HashMap;
use crate::shared::datetime::now_timestamp;

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

    pub fn level_lifetime(&self, price: Price, period: Period) -> Option<TimestampMS> {
        let events = self.ticks.get(&price)?;
        let (start_ts, end_ts) = period;

        // Найти первое ненулевое
        let first_nonzero = events.iter()
            .find(|ev| ev.timestamp >= start_ts && ev.timestamp < end_ts && ev.quantity > 0)
            .map(|ev| ev.timestamp)?;

        // Найти последнее ненулевое
        let last_nonzero = events.iter().rev()
            .find(|ev| ev.timestamp >= start_ts && ev.timestamp < end_ts && ev.quantity > 0)
            .map(|ev| ev.timestamp)?;

        Some(last_nonzero.saturating_sub(first_nonzero))
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
    pub fn get_best_bid(&self) -> Option<(Price, Quantity)> {
        self.bids.get_best()
    }

    pub fn get_best_ask(&self) -> Option<(Price, Quantity)> {
        self.asks.get_best()
    }

    pub fn get_position(&self, side: &Side, price: Price) -> Option<usize> {
        match side {
            Side::Buy => self.bids.get_position(price),
            Side::Sell => self.asks.get_position(price),
        }
    }

    pub fn get_bid_position(&self, price: Price) -> Option<usize> {
        self.bids.get_position(price)
    }

    pub fn get_ask_position(&self, price: Price) -> Option<usize> {
        self.asks.get_position(price)
    }

    pub fn handle_level_updated(&mut self, events: &[LevelUpdated]) {
        for event in events {
            match event.side {
                Side::Buy => self.bids.update(event.price, event.quantity),
                Side::Sell => self.asks.update(event.price, event.quantity),
            }
        }
    }
}

pub fn display_order_book(order_book: &OrderBook, depth: usize) {
    print!("\x1B[2J\x1B[H"); // очистить экран и курсор в начало

    println!("================= ORDER BOOK =================");
    println!("   BID (price x qty)         |     ASK (price x qty)");
    println!("-----------------------------------------------------");

    // 🔹 Соберём BID
    let mut bids: Vec<(Price, Quantity)> = order_book
        .bids
        .order
        .iter()
        .rev()
        .filter_map(|(&p, _)| order_book.bids.levels.get(&p).map(|&q| (p, q)))
        .take(depth)
        .collect();

    // 🔹 Соберём ASK
    let mut asks: Vec<(Price, Quantity)> = order_book
        .asks
        .order
        .iter()
        .filter_map(|(&p, _)| order_book.asks.levels.get(&p).map(|&q| (p, q)))
        .take(depth)
        .collect();

    // Выравниваем длины для красивого вывода
    let max_len = bids.len().max(asks.len());
    bids.resize(max_len, (0, 0));
    asks.resize(max_len, (0, 0));

    for i in 0..max_len {
        let (bp, bq) = bids[i];
        let (ap, aq) = asks[i];

        let bid_str = if bq > 0 {
            format!("{:>8} x {:<8}", bp, bq)
        } else {
            " ".repeat(18)
        };

        let ask_str = if aq > 0 {
            format!("{:>8} x {:<8}", ap, aq)
        } else {
            " ".repeat(18)
        };

        println!("  {}     |     {}", bid_str, ask_str);
    }
}
