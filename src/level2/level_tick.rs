use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::errors::{check_price, check_timestamp};
use crate::shared::{Price, TimestampMS};
use std::collections::VecDeque;

pub struct LevelTicks {
    price: Price,
    ticks: VecDeque<LevelUpdated>,
    max_buffer: usize,
    last_ts: TimestampMS,
}

impl LevelTicks {
    pub fn new(price: Price, max_buffer: usize) -> Self {
        Self {
            price,
            max_buffer,
            ticks: VecDeque::with_capacity(max_buffer + 1),
            last_ts: 0,
        }
    }

    pub fn push(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        check_price(event.price, self.price)?;
        check_timestamp(self.last_ts, event.timestamp)?;

        self.last_ts = event.timestamp;
        self.ticks.push_back(event);

        if self.ticks.len() > self.max_buffer {
            self.ticks.pop_front();
        }

        Ok(())
    }

    pub fn get_all(&self) -> &VecDeque<LevelUpdated> {
        &self.ticks
    }
}
