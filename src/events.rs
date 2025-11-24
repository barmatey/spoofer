use crate::bus::Event;

pub type Price = String;
pub type Quantity = String;

#[derive(Debug, Clone)]
pub enum Side{
    Buy,
    Sell,
}

#[derive(Debug)]
pub struct OrderCreated {
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
}
#[derive(Debug)]
pub struct OrderCancelled {
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
}

pub struct OrderUpdated {
    side: Side,
    price: Price,
    old_quantity: Quantity,
    new_quantity: Quantity,
}

impl Event for OrderCreated {}
impl Event for OrderCancelled {}
impl Event for OrderUpdated {}
