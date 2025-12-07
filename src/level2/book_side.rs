use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::errors::check_side;
use crate::shared::{Price, Quantity, Side};
use either::Either;
use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};

pub struct BookSide {
    levels: HashMap<Price, Quantity>,
    sorted_prices: BTreeSet<Price>,
    best_price: Option<Price>,
    side: Side,
    max_depth: usize,
}

impl BookSide {
    pub fn new(side: Side, max_depth: usize) -> Self {
        Self {
            levels: HashMap::new(),
            sorted_prices: BTreeSet::new(),
            side,
            max_depth,
            best_price: None,
        }
    }

    fn remove_level(&mut self, price: Price) {
        self.sorted_prices.remove(&price);
        self.levels.remove(&price);

        if price == self.best_price.unwrap() {
            self.best_price = match self.side {
                Side::Buy => self
                    .sorted_prices
                    .iter()
                    .rev()
                    .next()
                    .copied(),
                Side::Sell => self
                    .sorted_prices
                    .iter()
                    .next()
                    .copied()
            };
        }
    }

    fn insert_or_update_level(&mut self, price: Price, qty: Quantity) {
        match self.levels.entry(price) {
            Entry::Vacant(e) => {
                e.insert(qty);
                self.sorted_prices.insert(price);
                self.evict_extra_levels();
            }
            Entry::Occupied(mut e) => {
                e.insert(qty);
            }
        }

        match self.side {
            Side::Buy => self.best_price = Some(self.best_price.unwrap_or(0).max(price)),
            Side::Sell => self.best_price = Some(self.best_price.unwrap_or(Price::MAX).min(price)),
        }
    }

    fn evict_extra_levels(&mut self) {
        while self.sorted_prices.len() > self.max_depth {
            let remove_price = match self.side {
                Side::Buy => *self.sorted_prices.iter().next().unwrap(),
                Side::Sell => *self.sorted_prices.iter().next_back().unwrap(),
            };
            self.remove_level(remove_price);
        }
    }

    pub(crate) fn update(&mut self, event: &LevelUpdated) -> Result<(), Level2Error> {
        check_side(&self.side, &event.side)?;
        if event.quantity == 0 {
            self.remove_level(event.price);
        } else {
            self.insert_or_update_level(event.price, event.quantity);
        }
        Ok(())
    }

    pub(crate) fn update_or_miss(&mut self, event: &LevelUpdated) {
        if event.side == self.side {
            self.update(event).unwrap();
        }
    }

    pub fn best_price(&self) -> Option<Price> {
        self.best_price
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

    pub fn is_empty(&self) -> bool {
        self.sorted_prices.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level2::LevelUpdated;
    use crate::shared::utils::now_timestamp_ns;
    use crate::shared::Exchange;
    use std::sync::Arc;

    fn event(side: Side, price: Price, qty: Quantity) -> LevelUpdated {
        LevelUpdated {
            exchange: Exchange::Binance,
            ticker: Arc::new("no_matter".to_string()),
            side,
            price,
            quantity: qty,
            timestamp: 0,
            received: now_timestamp_ns(),
        }
    }

    #[test]
    fn test_insert_levels() {
        let mut book = BookSide::new(Side::Buy, 5);

        book.update(&event(Side::Buy, 100, 10)).unwrap();
        book.update(&event(Side::Buy, 105, 20)).unwrap();

        // Проверка уровня и best_price
        assert_eq!(book.best_price().unwrap(), 105);
        assert_eq!(book.levels[&100], 10);
        assert_eq!(book.levels[&105], 20);

        // Проверка итератора best_prices
        let prices: Vec<Price> = book.best_prices(2).copied().collect();
        assert_eq!(prices, vec![105, 100]);
    }

    #[test]
    fn test_update_level_quantity() {
        let mut book = BookSide::new(Side::Sell, 5);

        book.update(&event(Side::Sell, 200, 50)).unwrap();
        book.update(&event(Side::Sell, 150, 30)).unwrap();

        // Обновление существующего уровня
        book.update(&event(Side::Sell, 200, 60)).unwrap();

        assert_eq!(book.levels[&200], 60);
        assert_eq!(book.best_price().unwrap(), 150); // минимальная цена для Sell
    }

    #[test]
    fn test_remove_level() {
        let mut book = BookSide::new(Side::Buy, 5);

        book.update(&event(Side::Buy, 100, 10)).unwrap();
        book.update(&event(Side::Buy, 105, 20)).unwrap();

        // Удаляем лучший уровень
        book.update(&event(Side::Buy, 105, 0)).unwrap();
        assert_eq!(book.best_price().unwrap(), 100);
        assert!(!book.levels.contains_key(&105));

        // Удаляем последний уровень
        book.update(&event(Side::Buy, 100, 0)).unwrap();
        assert_eq!(book.best_price(), None);
        assert!(book.levels.is_empty());
    }

    #[test]
    fn test_evict_extra_levels() {
        let mut book = BookSide::new(Side::Sell, 3);

        book.update(&event(Side::Sell, 100, 10)).unwrap();
        book.update(&event(Side::Sell, 105, 10)).unwrap();
        book.update(&event(Side::Sell, 110, 10)).unwrap();
        book.update(&event(Side::Sell, 115, 10)).unwrap(); // должен вызвать eviction

        assert_eq!(book.sorted_prices.len(), 3);
        // Для Sell удаляется самый дорогой
        assert!(!book.levels.contains_key(&115));
        assert_eq!(book.best_price().unwrap(), 100);
    }

    #[test]
    fn test_update_or_miss() {
        let mut book = BookSide::new(Side::Buy, 5);

        // Сторона совпадает
        book.update_or_miss(&event(Side::Buy, 50, 5));
        assert_eq!(book.levels[&50], 5);

        // Сторона не совпадает — ничего не делаем
        book.update_or_miss(&event(Side::Sell, 60, 10));
        assert!(!book.levels.contains_key(&60));
    }

    #[test]
    fn test_best_prices_depth() {
        let mut book = BookSide::new(Side::Buy, 5);

        book.update(&event(Side::Buy, 100, 10)).unwrap();
        book.update(&event(Side::Buy, 105, 20)).unwrap();
        book.update(&event(Side::Buy, 110, 5)).unwrap();

        // depth = 2
        let prices: Vec<Price> = book.best_prices(2).copied().collect();
        assert_eq!(prices, vec![110, 105]);

        // depth больше, чем количество уровней
        let prices: Vec<Price> = book.best_prices(10).copied().collect();
        assert_eq!(prices, vec![110, 105, 100]);
    }

    #[test]
    fn test_check_side_error() {
        let mut book = BookSide::new(Side::Buy, 5);

        let res = book.update(&event(Side::Sell, 100, 10));
        assert!(res.is_err());
    }

    #[test]
    fn test_is_empty_initial() {
        let book = BookSide::new(Side::Buy, 5);
        assert!(book.is_empty());
    }

    #[test]
    fn test_is_empty_after_insert() {
        let mut book = BookSide::new(Side::Buy, 5);
        book.update(&event(Side::Buy, 100, 10)).unwrap();
        assert!(!book.is_empty());
    }

    #[test]
    fn test_is_empty_after_remove_all() {
        let mut book = BookSide::new(Side::Buy, 5);
        book.update(&event(Side::Buy, 100, 10)).unwrap();
        book.update(&event(Side::Buy, 105, 20)).unwrap();

        // Удаляем все уровни
        book.update(&event(Side::Buy, 100, 0)).unwrap();
        book.update(&event(Side::Buy, 105, 0)).unwrap();

        assert!(book.is_empty());
    }

    #[test]
    fn test_best_price_after_multiple_removes() {
        let mut book = BookSide::new(Side::Buy, 5);

        // Вставляем несколько уровней
        book.update(&event(Side::Buy, 100, 10)).unwrap();
        book.update(&event(Side::Buy, 105, 20)).unwrap();
        book.update(&event(Side::Buy, 110, 15)).unwrap();

        // Проверяем начальный best_price
        assert_eq!(book.best_price().unwrap(), 110);

        // Удаляем текущий лучший уровень
        book.update(&event(Side::Buy, 110, 0)).unwrap();
        assert_eq!(book.best_price().unwrap(), 105);

        // Удаляем следующий лучший уровень
        book.update(&event(Side::Buy, 105, 0)).unwrap();
        assert_eq!(book.best_price().unwrap(), 100);

        // Удаляем последний уровень
        book.update(&event(Side::Buy, 100, 0)).unwrap();
        assert_eq!(book.best_price(), None);
        assert!(book.levels.is_empty());
    }
    #[test]
    fn test_best_price_sell_after_multiple_removes() {
        let mut book = BookSide::new(Side::Sell, 5);

        // Вставляем несколько уровней
        book.update(&event(Side::Sell, 200, 10)).unwrap();
        book.update(&event(Side::Sell, 180, 20)).unwrap();
        book.update(&event(Side::Sell, 190, 15)).unwrap();

        // Проверяем начальный best_price (для Sell — минимальная цена)
        assert_eq!(book.best_price().unwrap(), 180);

        // Удаляем текущий лучший уровень (минимальный)
        book.update(&event(Side::Sell, 180, 0)).unwrap();
        assert_eq!(book.best_price().unwrap(), 190);

        // Удаляем следующий лучший уровень
        book.update(&event(Side::Sell, 190, 0)).unwrap();
        assert_eq!(book.best_price().unwrap(), 200);

        // Удаляем последний уровень
        book.update(&event(Side::Sell, 200, 0)).unwrap();
        assert_eq!(book.best_price(), None);
        assert!(book.levels.is_empty());
    }
}
