use crate::shared::{Quantity, Side, TimestampMS};

#[derive(Debug)]
pub enum BookStatsError {
    DeepError,
    SortError,
}

pub struct Snap {
    pub level: usize,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub side: Side,
}

struct Track {
    snaps: Vec<Vec<Snap>>,
}

impl Track {
    pub fn new(max_depth: usize) -> Self {
        let mut snaps = Vec::with_capacity(max_depth);
        for _ in 0..max_depth {
            snaps.push(Vec::new());
        }
        Self { snaps }
    }
    pub fn push(&mut self, snap: Snap) -> Result<(), BookStatsError> {
        if snap.level >= self.snaps.len() {
            return Err(BookStatsError::DeepError);
        }

        if let Some(last_snap) = self.snaps[snap.level].last() {
            if snap.timestamp < last_snap.timestamp {
                return Err(BookStatsError::SortError);
            }
        }

        self.snaps[snap.level].push(snap);
        Ok(())
    }

    pub fn get_average_quantity(
        &self,
        level: usize,
        period: TimestampMS,
    ) -> Result<u128, BookStatsError> {
        if level >= self.snaps.len() {
            return Err(BookStatsError::DeepError);
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

pub struct BookStats {
    bids: Track,
    asks: Track,
}

impl BookStats {
    pub fn new(max_depth: usize) -> Self {
        Self {
            bids: Track::new(max_depth),
            asks: Track::new(max_depth),
        }
    }

    pub fn push(&mut self, snap: Snap) -> Result<(), BookStatsError> {
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
    ) -> Result<u128, BookStatsError> {
        match side {
            Side::Buy => self.bids.get_average_quantity(level, period),
            Side::Sell => self.asks.get_average_quantity(level, period),
        }
    }
}

