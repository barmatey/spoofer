#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ClickHouseError")]
    ClickHouseError(#[from] clickhouse::error::Error),

    #[error("DepthError: {0}")]
    DepthError(#[from] crate::level2::Level2Error),

    #[error("TradeError: {0}")]
    TradeError(#[from] crate::trade::TradeError),


    #[error("SignalError: {0}")]
    SignalError(#[from] crate::signal::error::Error),
}
