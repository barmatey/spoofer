use std::sync::Arc;
use crate::shared::{Price, Quantity, Side, TimestampMS};

#[derive(Debug, Clone)]
pub struct LevelUpdated {
    pub exchange: Arc<String>,
    pub ticker: Arc<String>,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
}
