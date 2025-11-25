use crate::domain::events::{Quantity, Side, TimestampMS};

#[derive(Debug)]
pub enum SnapError {
    DeepError(String),
    SortError(String),
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
            let err = format!(
                "You are trying push level {} while {} is maximum",
                snap.level,
                self.snaps.len() - 1
            );
            return Err(SnapError::DeepError(err));
        }

        if let Some(last_snap) = self.snaps[snap.level].last() {
            if snap.timestamp < last_snap.timestamp {
                let err = "You are trying to push a snap with an earlier timestamp".to_string();
                return Err(SnapError::SortError(err));
            }
        }

        self.snaps[snap.level].push(snap);
        Ok(())
    }

    pub fn get_average_quantity(&self, level: usize, period: TimestampMS) -> Result<u128, SnapError> {
        if level >= self.snaps.len() {
            let err = format!("Level {} does not exist", level);
            return Err(SnapError::DeepError(err));
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
