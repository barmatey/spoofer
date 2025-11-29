use thiserror::Error;
use url::ParseError as UrlParseError;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("Trade parsing error")]
    ParsingTradeError,

    #[error("Level2 parsing error")]
    ParsingLevel2Error,

    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] UrlParseError),
}

#[derive(Debug, Error)]
pub enum WebsocketError {
    #[error("Websocket connection failed")]
    ConnectionFailed,
}

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("Parsing failed: {0}")]
    ParsingError(#[from] ParsingError),

    #[error("Websocket disconnected")]
    WebsocketError(#[from] WebsocketError),
}
