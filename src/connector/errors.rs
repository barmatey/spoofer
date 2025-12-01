#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    #[error("Level2 parsing error")]
    SerdeError(#[from] serde_json::Error),

    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("MessageParsingError error: {0}")]
    MessageParsingError(String),

    #[error("Converting error: {0}")]
    ConvertingError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum WebsocketError {
    #[error("Websocket connection failed")]
    ConnectionFailed,

    #[error("Send message failed")]
    SendMessageFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("KrakenError")]
    KrakenError(String),

    #[error("BinanceError")]
    BinanceError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parsing failed: {0}")]
    ParsingError(#[from] ParsingError),

    #[error("Websocket disconnected")]
    WebsocketError(#[from] WebsocketError),

    #[error("Builder Error")]
    BuilderError(String),

    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Exchange Error: {0}")]
    ExchangeError(#[from] ExchangeError),

    #[error("InternalError")]
    InternalError(String),
}

pub type ErrorHandler = Box<dyn Fn(&Error)>;
