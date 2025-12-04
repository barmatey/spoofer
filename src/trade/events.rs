use std::sync::Arc;
use crate::shared::{Price, Quantity, Side, TimestampMS};

#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub exchange: Arc<String>,
    pub ticker: Arc<String>,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub market_maker: Side,
}

