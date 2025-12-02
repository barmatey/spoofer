use crate::level2::level_tick::LevelTicks;
use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::errors::{check_side};
use crate::shared::{Price, Side};
use either::Either;
use std::collections::{BTreeSet, HashMap, VecDeque};
use once_cell::sync::Lazy;

static EMPTY_TICKS: Lazy<VecDeque<LevelUpdated>> = Lazy::new(|| VecDeque::new());


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
                Side::Sell => *self.sorted_prices.iter().rev().next().unwrap(), // наибольшая цена
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
