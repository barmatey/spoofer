use crate::shared::errors::BaseError;

#[derive(Debug, thiserror::Error)]
pub enum TradeError{
    #[error("EventError")]
    EventError(#[from] BaseError),
}