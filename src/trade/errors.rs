#[derive(Debug, thiserror::Error)]
pub enum TradeError{
    #[error("OutdatedEvent")]
    OutdatedEvent(String),

    #[error("IncompatibleTicker")]
    IncompatibleTicker(String),

    #[error("IncompatibleExchange")]
    IncompatibleExchange(String),
}