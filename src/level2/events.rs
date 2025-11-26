use crate::shared::{Price, Quantity, Side, TimestampMS};

#[derive(Debug)]
pub struct LevelUpdated {
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
}
