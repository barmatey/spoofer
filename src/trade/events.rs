use crate::shared::{Price, Quantity, Side, TimestampMS};

#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub exchange: String,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub market_maker: Side,
}

