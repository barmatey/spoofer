pub type Price = u128;
pub type Quantity = u128;

pub type TimestampMS = u64;

#[derive(Debug, Clone)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug)]
pub struct LevelUpdated {
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
}

#[derive(Debug)]
pub struct TradeEvent {
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub is_buyer_maker: bool,
}

