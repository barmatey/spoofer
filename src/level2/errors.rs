use crate::shared::errors::BaseError;
use clickhouse::error::Error as CHError;

#[derive(Debug, thiserror::Error)]
pub enum Level2Error {
    #[error("EventError")]
    EventError(#[from] BaseError),

    #[error("RepoError: {0}")]
    RepoError(#[from] CHError),
}
