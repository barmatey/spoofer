use crate::shared::{Price, Quantity, TimestampMS};

#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub is_buyer_maker: bool,
}

