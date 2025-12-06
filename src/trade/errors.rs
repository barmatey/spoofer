use crate::shared::errors::BaseError;

#[derive(Debug, thiserror::Error)]
pub enum TradeError{
    #[error("EventError")]
    EventError(#[from] BaseError),


    #[error("RepoError: {0}")]
    RepoError(#[from] clickhouse::error::Error),
}