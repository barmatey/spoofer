use std::any::{Any, TypeId};

pub trait Event: Send + Sync+ Any {
    fn as_any(&self) -> &dyn Any;
}
