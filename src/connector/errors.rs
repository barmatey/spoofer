use serde_json::Error;
use thiserror::Error;
use url::ParseError as UrlParseError;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("Level2 parsing error")]
    SerdeParseError(#[from] Error),

    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] UrlParseError),

    #[error("Converting error: {0}")]
    MessageParsingError(String),

    #[error("Converting error: {0}")]
    ConvertingError(String),
}

#[derive(Debug, Error)]
pub enum WebsocketError {
    #[error("Websocket connection failed")]
    ConnectionFailed,

    #[error("Send message failed")]
    SendMessageFailed,
}

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("Parsing failed: {0}")]
    ParsingError(#[from] ParsingError),

    #[error("Websocket disconnected")]
    WebsocketError(#[from] WebsocketError),
}
