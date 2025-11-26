use crate::shared::{Price, TimestampMS};

pub struct SpooferDetected {
    pub price: Price,
    pub score: u16,
    pub timestamp: TimestampMS,
}
