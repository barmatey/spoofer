use crate::shared::errors::BaseError;

#[derive(Debug, thiserror::Error)]
pub enum Level2Error {
    #[error("EventError")]
    EventError(#[from] BaseError),
}
