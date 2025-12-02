#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("OutdatedEvent")]
    OutdatedEvent(String),

    #[error("IncompatibleTicker")]
    IncompatibleTicker(String),

    #[error("IncompatibleExchange")]
    IncompatibleExchange(String),
}
