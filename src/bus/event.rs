use std::any::{Any, TypeId};

pub trait Event: Send + Sync {
    fn as_any(&self) -> &dyn Any;
}
