use std::any::{Any, TypeId};
use crate::bus::Event;

pub type Price = String;
pub type Quantity = String;

#[derive(Debug, Clone)]
pub enum Side{
    Buy,
    Sell,
}


#[derive(Debug)]
pub struct LevelUpdated {
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
}


impl Event for LevelUpdated {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<LevelUpdated>()
    }
}
