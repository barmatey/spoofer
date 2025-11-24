use crate::bus::Event;

pub type Price = String;
pub type Quantity = String;

pub struct OrderCreated {}

pub struct OrderCancelled {
    side: String,
    price: String,
    quantity: String,
}

pub struct OrderUpdated {
    side: String,
    price: String,
    old_quantity: String,
    new_quantity: String,
}

impl Event for OrderCreated {}
impl Event for OrderCancelled {}
impl Event for OrderUpdated {}
