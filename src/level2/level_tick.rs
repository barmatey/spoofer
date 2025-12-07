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

    pub fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error> {
        check_price(event.price, self.price)?;
        check_timestamp(self.last_ts, event.timestamp)?;

        self.last_ts = event.timestamp;
        self.ticks.push_back(event);

        if self.ticks.len() > self.max_buffer {
            self.ticks.pop_front();
        }

        Ok(())
    }

    pub fn update_or_miss(&mut self, event: LevelUpdated) {
        if event.price == self.price && event.timestamp >= self.last_ts {
            self.update(event).unwrap();
        }
    }

    pub fn get_all(&self) -> &VecDeque<LevelUpdated> {
        &self.ticks
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;
    use crate::level2::LevelUpdated;
    use crate::shared::{Exchange, Quantity};
    use crate::shared::utils::now_timestamp_ns;

    fn make_event(price: Price, timestamp: TimestampMS, quantity: Quantity) -> LevelUpdated {
        LevelUpdated {
            price,
            quantity,
            timestamp,
            side: crate::shared::Side::Buy,
            ticker: Arc::new("BTC/USDT".to_string()),
            exchange: Exchange::Binance,
            received:  now_timestamp_ns(),
        }
    }

    #[test]
    fn test_ticks_push_and_order() {
        let mut lvl = LevelTicks::new(100, 3);

        lvl.update(make_event(100, 1, 5)).unwrap();
        lvl.update(make_event(100, 2, 10)).unwrap();
        lvl.update(make_event(100, 3, 15)).unwrap();

        let events = lvl.get_all();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].timestamp, 1);
        assert_eq!(events[1].timestamp, 2);
        assert_eq!(events[2].timestamp, 3);
    }

    #[test]
    fn test_ticks_max_buffer_eviction() {
        let mut lvl = LevelTicks::new(100, 2);

        lvl.update(make_event(100, 1, 5)).unwrap();
        lvl.update(make_event(100, 2, 10)).unwrap();
        lvl.update(make_event(100, 3, 15)).unwrap();

        let events = lvl.get_all();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].timestamp, 2); // старое событие удалено
        assert_eq!(events[1].timestamp, 3);
    }

    #[test]
    fn test_ticks_wrong_price() {
        let mut lvl = LevelTicks::new(100, 2);
        let err = lvl.update(make_event(1010, 1, 5));
        assert!(err.is_err());
    }

    #[test]
    fn test_ticks_timestamp_order() {
        let mut lvl = LevelTicks::new(100, 3);
        lvl.update(make_event(100, 2, 5)).unwrap();

        let err = lvl.update(make_event(100, 1, 5));
        assert!(err.is_err());

    }

    #[test]
    fn test_ticks_empty() {
        let lvl = LevelTicks::new(100, 2);
        let events = lvl.get_all();
        assert!(events.is_empty());
    }
}
