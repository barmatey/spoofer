use crate::domain::events::{Quantity, Side, TimestampMS};

#[derive(Debug)]
pub enum SnapError {
    DeepError,
    SortError,
}

pub struct Snap {
    pub level: usize,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub side: Side,
}

struct Level {
    snaps: Vec<Vec<Snap>>,
}

impl Level {
    pub fn new(max_depth: usize) -> Self {
        let mut snaps = Vec::with_capacity(max_depth);
        for _ in 0..max_depth {
            snaps.push(Vec::new());
        }
        Self { snaps }
    }
    pub fn push(&mut self, snap: Snap) -> Result<(), SnapError> {
        if snap.level >= self.snaps.len() {
            return Err(SnapError::DeepError);
        }

        if let Some(last_snap) = self.snaps[snap.level].last() {
            if snap.timestamp < last_snap.timestamp {
                return Err(SnapError::SortError);
            }
        }

        self.snaps[snap.level].push(snap);
        Ok(())
    }

    pub fn get_average_quantity(
        &self,
        level: usize,
        period: TimestampMS,
    ) -> Result<u128, SnapError> {
        if level >= self.snaps.len() {
            return Err(SnapError::DeepError);
        }

        let snaps = &self.snaps[level];
        if snaps.is_empty() {
            return Ok(0);
        }

        // Берем текущее время как timestamp последнего снапа на уровне
        let last_timestamp = snaps.last().unwrap().timestamp;
        let threshold = last_timestamp.saturating_sub(period);

        // Идем с конца, так как данные отсортированы по времени
        let mut sum: u128 = 0;
        let mut count: u128 = 0;

        for snap in snaps.iter().rev() {
            if snap.timestamp < threshold {
                break; // дальше все старее периода
            }
            sum += snap.quantity as u128;
            count += 1;
        }

        if count == 0 {
            Ok(0)
        } else {
            Ok(sum / count)
        }
    }
}

pub struct OrderStat {
    bids: Level,
    asks: Level,
}

impl OrderStat {
    pub fn new(max_depth: usize) -> Self {
        Self {
            bids: Level::new(max_depth),
            asks: Level::new(max_depth),
        }
    }

    pub fn push(&mut self, snap: Snap) -> Result<(), SnapError> {
        match snap.side {
            Side::Buy => self.bids.push(snap),
            Side::Sell => self.asks.push(snap),
        }
    }

    pub fn get_average_quantity(
        &self,
        side: Side,
        level: usize,
        period: TimestampMS,
    ) -> Result<u128, SnapError> {
        match side {
            Side::Buy => self.bids.get_average_quantity(level, period),
            Side::Sell => self.asks.get_average_quantity(level, period),
        }
    }
}

mod tests {
    use crate::domain::events::Side;
    use crate::domain::order_stat::SnapError;
    use crate::domain::{OrderStat, Snap};

    #[test]
    fn test_get_average_quantity() {
        let s1 = Snap {
            level: 0,
            quantity: 6,
            timestamp: 1,
            side: Side::Buy,
        };
        let s2 = Snap {
            level: 0,
            quantity: 2,
            timestamp: 20,
            side: Side::Buy,
        };
        let s3 = Snap {
            level: 0,
            quantity: 4,
            timestamp: 30,
            side: Side::Buy,
        };
        let mut foo = OrderStat::new(1);
        foo.push(s1).unwrap();
        foo.push(s2).unwrap();
        foo.push(s3).unwrap();
        let left = foo.get_average_quantity(Side::Buy, 0, 25).unwrap();
        assert_eq!(left, 3);
    }

    #[test]
    fn test_push_snap_with_exceed_level() {
        let s1 = Snap {
            level: 1,
            quantity: 6,
            timestamp: 1,
            side: Side::Buy,
        };
        let mut foo = OrderStat::new(1);
        let left = foo.push(s1);
        assert!(left.is_err());
    }

    #[test]
    fn push_earlier_snap_after_older_one() {
        let s1 = Snap {
            level: 0,
            quantity: 6,
            timestamp: 2,
            side: Side::Buy,
        };
        let s2 = Snap {
            level: 0,
            quantity: 6,
            timestamp: 1,
            side: Side::Buy,
        };
        let mut foo = OrderStat::new(1);
        foo.push(s1).unwrap();
        let left = foo.push(s2);
        assert!(left.is_err());
    }
}
