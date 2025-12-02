#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    #[error("OutdatedEvent")]
    OutdatedError(String),

    #[error("IncompatibleTicker")]
    IncompatibleTicker(String),

    #[error("IncompatibleExchange")]
    IncompatibleExchange(String),

    #[error("IncompatibleSide")]
    IncompatibleSide(String),
}
