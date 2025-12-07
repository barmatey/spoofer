use crate::shared::{Exchange, Price, Quantity, Side, TimestampMS, TimestampNS};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LevelUpdated {
    pub exchange: Exchange,
    pub ticker: Arc<String>,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub received: TimestampNS,
}
