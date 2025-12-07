use std::sync::Arc;
use crate::shared::{Exchange, Price, Quantity, Side, TimestampMS};

#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub exchange: Exchange,
    pub ticker: Arc<String>,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub market_maker: Side,
}

