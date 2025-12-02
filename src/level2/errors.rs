#[derive(Debug, thiserror::Error)]
pub enum Level2Error {
    #[error("IncompatibleSide")]
    IncompatibleSide(String),

    #[error("OutdatedEvent")]
    OutdatedEvent(String),

    #[error("IncompatibleTicker")]
    IncompatibleTicker(String),
    
    #[error("IncompatibleExchange")]
    IncompatibleExchange(String),
}
