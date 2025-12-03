use crate::level2::level_tick::LevelTicks;
use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::errors::check_side;
use crate::shared::{Price, Side};
use either::Either;
use std::collections::{BTreeSet, HashMap, VecDeque};

static EMPTY_TICKS: VecDeque<LevelUpdated> = VecDeque::new();

pub struct BookSide {
    ticks: HashMap<Price, LevelTicks>,
    sorted_prices: BTreeSet<Price>,
    side: Side,
    max_levels: usize,
    max_ticks_per_price: usize,
}

impl BookSide {
    pub fn new(side: Side, max_levels: usize, max_ticks_per_price: usize) -> Self {
        Self {
            ticks: HashMap::new(),
            sorted_prices: BTreeSet::new(),
            side,
            max_levels,
            max_ticks_per_price,
        }
    }

    fn evict_extra_levels(&mut self) {
        while self.sorted_prices.len() > self.max_levels {
            let remove_price = match self.side {
                Side::Buy => *self.sorted_prices.iter().next().unwrap(), // наименьшая цена
                Side::Sell => *self.sorted_prices.iter().next_back().unwrap(), // наибольшая цена
            };
            self.sorted_prices.remove(&remove_price);
            self.ticks.remove(&remove_price);
        }
    }

    fn get_or_create_level(&mut self, price: Price) -> &mut LevelTicks {
        if !self.ticks.contains_key(&price) {
            self.sorted_prices.insert(price);
        }

        self.ticks
            .entry(price)
            .or_insert_with(|| LevelTicks::new(price, self.max_ticks_per_price))
    }

    pub(crate) fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        check_side(&self.side, &event.side)?;
        let price = event.price;
        self.get_or_create_level(price).update(event)?;
        self.evict_extra_levels();
        Ok(())
    }
    pub(crate) fn update_or_miss(&mut self, event: LevelUpdated) {
        if event.side == self.side {
            let price = event.price;
            self.get_or_create_level(price).update_or_miss(event);
            self.evict_extra_levels();
        }
    }
    pub fn level_ticks(&self, price: Price) -> &VecDeque<LevelUpdated> {
        self.ticks
            .get(&price)
            .map(|v| v.get_all())
            .unwrap_or(&EMPTY_TICKS)
    }

    pub fn best_prices(&self, depth: usize) -> impl Iterator<Item = &Price> {
        let iter = match self.side {
            Side::Buy => Either::Left(self.sorted_prices.iter().rev()),
            Side::Sell => Either::Right(self.sorted_prices.iter()),
        };
        iter.take(depth)
    }

    pub fn side(&self) -> &Side {
        &self.side
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level2::{LevelUpdated};
    use crate::shared::{Price, Side};

    fn ev(price: Price, ts: u64, side: Side) -> LevelUpdated {
        LevelUpdated {
            price,
            quantity: 1,
            timestamp: ts,
            side,
            ticker: "btc/usdt".to_string(),
            exchange: "Binance".to_string(),
        }
    }

    #[test]
    fn test_add_single_level() {
        let mut bs = BookSide::new(Side::Buy, 10, 100);
        bs.update(ev(100, 1, Side::Buy)).unwrap();

        let lv = bs.level_ticks(100);
        assert_eq!(lv.len(), 1);
        assert_eq!(lv[0].timestamp, 1);
    }

    #[test]
    fn test_wrong_side_rejected() {
        let mut bs = BookSide::new(Side::Buy, 10, 100);

        let result = bs.update(ev(100, 1, Side::Sell));
        assert!(result.is_err())
    }

    #[test]
    fn test_update_or_miss_ignores_wrong_side() {
        let mut bs = BookSide::new(Side::Buy, 10, 100);

        bs.update_or_miss(ev(100, 1, Side::Sell));
        assert!(bs.level_ticks(100).is_empty());
    }

    #[test]
    fn test_best_prices_buy() {
        let mut bs = BookSide::new(Side::Buy, 10, 100);
        bs.update(ev(100, 1, Side::Buy)).unwrap();
        bs.update(ev(105, 2, Side::Buy)).unwrap();
        bs.update(ev(103, 3, Side::Buy)).unwrap();

        let best: Vec<_> = bs.best_prices(2).copied().collect();
        assert_eq!(best, vec![105, 103]); // у Buy — убывающий порядок
    }

    #[test]
    fn test_best_prices_sell() {
        let mut bs = BookSide::new(Side::Sell, 10, 100);
        bs.update(ev(100, 1, Side::Sell)).unwrap();
        bs.update(ev(105, 2, Side::Sell)).unwrap();
        bs.update(ev(103, 3, Side::Sell)).unwrap();

        let best: Vec<_> = bs.best_prices(2).copied().collect();
        assert_eq!(best, vec![100, 103]); // у Sell — возрастающий порядок
    }

    #[test]
    fn test_max_levels_enforced_buy() {
        let mut bs = BookSide::new(Side::Buy, 2, 100);

        bs.update(ev(100, 1, Side::Buy)).unwrap();
        bs.update(ev(101, 2, Side::Buy)).unwrap();
        bs.update(ev(102, 3, Side::Buy)).unwrap(); // должен вытеснить наименьшую цену = 100

        assert!(bs.level_ticks(100).is_empty());
        assert!(!bs.level_ticks(101).is_empty());
        assert!(!bs.level_ticks(102).is_empty());
    }

    #[test]
    fn test_max_levels_enforced_sell() {
        let mut bs = BookSide::new(Side::Sell, 2, 100);

        bs.update(ev(100, 1, Side::Sell)).unwrap();
        bs.update(ev(101, 2, Side::Sell)).unwrap();
        bs.update(ev(102, 3, Side::Sell)).unwrap(); // должен вытеснить наибольшую цену = 102

        assert!(bs.level_ticks(102).is_empty());
        assert!(!bs.level_ticks(100).is_empty());
        assert!(!bs.level_ticks(101).is_empty());
    }

    #[test]
    fn test_max_ticks_per_price() {
        let mut bs = BookSide::new(Side::Buy, 10, 2);

        bs.update(ev(100, 1, Side::Buy)).unwrap();
        bs.update(ev(100, 2, Side::Buy)).unwrap();
        bs.update(ev(100, 3, Side::Buy)).unwrap(); // должен вытеснить самый старый

        let ticks = bs.level_ticks(100);
        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0].timestamp, 2);
        assert_eq!(ticks[1].timestamp, 3);
    }

    #[test]
    fn test_level_ticks_missing_returns_empty() {
        let bs = BookSide::new(Side::Buy, 10, 100);
        let lv = bs.level_ticks(999);
        assert!(lv.is_empty());
    }

    #[test]
    fn test_update_or_miss_adds_miss_event() {
        let mut bs = BookSide::new(Side::Buy, 10, 10);

        bs.update_or_miss(ev(100, 1, Side::Buy));
        bs.update_or_miss(ev(100, 2, Side::Buy));

        let ticks = bs.level_ticks(100);
        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0].timestamp, 1);
        assert_eq!(ticks[1].timestamp, 2);
    }

    #[test]
    fn test_best_price_ignores_zero_quantity() {
        let mut bs = BookSide::new(Side::Buy, 10, 100);
        bs.update(ev(101, 1, Side::Buy)).unwrap();
        let mut ev_zero = ev(102, 2, Side::Buy);
        ev_zero.quantity = 0;
        bs.update(ev_zero).unwrap();
        bs.update(ev(100, 3, Side::Buy)).unwrap();
        let best: Vec<_> = bs.best_prices(3).copied().collect();
        assert_eq!(best, vec![101, 100]);
    }
}
