use crate::bus::Event;

pub trait Handler{
    type E: Event;

    async fn handle(event: Self::E) {

    }
}