use crate::connector::errors::{ConnectorError, ParsingError, WebsocketError};
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio::net::TcpStream;
use crate::shared::TimestampMS;

pub type Connection = (
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
);

pub async fn connect_websocket(url: &str) -> Result<Connection, ConnectorError> {
    println!("🔗 Connecting to WS: {}", url);

    let parsed_url =
        Url::parse(url).map_err(|e| ConnectorError::from(ParsingError::UrlParseError(e)))?;

    let (ws_stream, _) = connect_async(parsed_url)
        .await
        .map_err(|_| ConnectorError::WebsocketError(WebsocketError::ConnectionFailed))?;

    Ok(ws_stream.split())
}

pub type ConnectionSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub async fn send_ws_message(sink: &mut ConnectionSink, msg: Message) -> Result<(), ConnectorError> {
    sink.send(msg)
        .await
        .map_err(|_| ConnectorError::WebsocketError(WebsocketError::SendMessageFailed))
}


pub fn parse_json<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, ConnectorError> {
    serde_json::from_str::<T>(s).map_err(|e| ConnectorError::from(ParsingError::SerdeParseError(e)))
}

pub fn parse_number(s: &str) -> Result<f64, ParsingError> {
    s.parse::<f64>()
        .map_err(|e| ParsingError::ConvertingError(format!("{}", e)))
}


pub fn parse_timestamp(s: &str) -> Result<TimestampMS, ParsingError>{
    s.parse::<TimestampMS>()
        .map_err(|e| ParsingError::ConvertingError(format!("{}", e)))
}