use crate::shared::event::EventError;

#[derive(Debug, thiserror::Error)]
pub enum Level2Error {
    #[error("IncompatibleSide")]
    IncompatibleSide(String),

    #[error("OutdatedEvent")]
    EventError(#[from] EventError),
}
