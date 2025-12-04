use crate::shared::event::errors::EventError;
use crate::shared::event::traits::Event;

pub struct EventRepo {}

impl EventRepo {
    pub async fn add(&self, event: &dyn Event) -> Result<(), EventError> {
        Ok(())
    }

    pub async fn get_all<T: Event>(&self) -> Result<Vec<Box<dyn Event>>, EventError> {
        Ok(vec![])
    }
}
