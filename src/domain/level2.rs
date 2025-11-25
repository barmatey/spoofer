use crate::domain::events::{Price, Quantity, Side};
use std::collections::{BTreeMap, HashMap};

struct Level {
    active: BTreeMap<Price, ()>,
    levels: HashMap<Price, Quantity>,
    side: Side,
}

impl Level {
    pub fn new(side: Side) -> Self {
        Self {
            active: BTreeMap::new(),
            levels: HashMap::new(),
            side,
        }
    }

    /// Обновление уровня: если quantity=0, удаляем
    pub fn update(&mut self, price: Price, quantity: Quantity) {
        if quantity == 0 {
            self.levels.remove(&price);
            self.active.remove(&price);
        } else {
            let is_new = !self.levels.contains_key(&price);
            self.levels.insert(price, quantity);
            if is_new {
                self.active.insert(price, ());
            }
        }
    }

    /// Лучший уровень (максимальная цена для BID, минимальная для ASK)
    pub fn get_best(&self) -> Option<(Price, Quantity)> {
        match self.side {
            Side::Buy => self
                .active
                .iter()
                .next_back()
                .and_then(|(&price, _)| self.levels.get(&price).map(|&q| (price, q))),
            Side::Sell => self
                .active
                .iter()
                .next()
                .and_then(|(&price, _)| self.levels.get(&price).map(|&q| (price, q))),
        }
    }

    /// Позиция уровня: 0 = лучший, 1 = следующий и т.д.
    pub fn get_position(&self, price: Price) -> Option<u16> {
        if !self.levels.contains_key(&price) {
            return None;
        }

        let mut pos: u16 = 0;

        match self.side {
            Side::Buy => {
                // Bid: идем от максимальной цены к минимальной
                for &p in self.active.keys().rev() {
                    if p == price {
                        return Some(pos);
                    }
                    pos += 1;
                }
            }
            Side::Sell => {
                // Ask: идем от минимальной цены к максимальной
                for &p in self.active.keys() {
                    if p == price {
                        return Some(pos);
                    }
                    pos += 1;
                }
            }
        }

        None
    }
}

pub struct OrderBook {
    bids: Level,
    asks: Level,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: Level::new(Side::Buy),
            asks: Level::new(Side::Sell),
        }
    }
    pub fn get_best_bid(&self) -> Option<(Price, Quantity)> {
        self.bids.get_best()
    }

    pub fn get_best_ask(&self) -> Option<(Price, Quantity)> {
        self.asks.get_best()
    }

    pub fn get_bid_position(&self, price: Price) -> Option<u16> {
        self.bids.get_position(price)
    }

    pub fn get_ask_position(&self, price: Price) -> Option<u16> {
        self.asks.get_position(price)
    }

    pub fn update_bid(&mut self, price: Price, quantity: Quantity) {
        self.bids.update(price, quantity)
    }

    pub fn update_ask(&mut self, price: Price, quantity: Quantity) {
        self.asks.update(price, quantity)
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
        .active
        .iter()
        .rev()
        .filter_map(|(&p, _)| order_book.bids.levels.get(&p).map(|&q| (p, q)))
        .take(depth)
        .collect();

    // 🔹 Соберём ASK
    let mut asks: Vec<(Price, Quantity)> = order_book
        .asks
        .active
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
