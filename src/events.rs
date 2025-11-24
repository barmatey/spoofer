use crate::bus::Event;
use std::any::Any;

pub type Price = u128;
pub type Quantity = u128;

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
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct TradeEvent {
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: u64,
    pub is_buyer_maker: bool,
}

impl Event for LevelUpdated {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Event for TradeEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
