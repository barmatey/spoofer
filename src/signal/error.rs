#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("DatabaseError: {0}")]
    DatabaseError(#[from] clickhouse::error::Error),
}
