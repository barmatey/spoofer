use crate::connector::errors::{ConnectorError, ParsingError, WebsocketError};
use crate::shared::TimestampMS;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use futures_util::StreamExt;
use std::time::Duration;
use chrono::{DateTime, Utc};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use url::Url;

pub type Connection = (
    SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
);

pub async fn connect_websocket(url: &str) -> Result<Connection, ConnectorError> {
    println!("🔗 Connecting to WS: {}", url);

    let parsed_url =
        Url::parse(url).map_err(|e| ConnectorError::from(ParsingError::UrlParseError(e)))?;

    let (ws_stream, _) = connect_async(parsed_url)
        .await
        .map_err(|_| ConnectorError::WebsocketError(WebsocketError::ConnectionFailed))?;

    println!("🟢 Successfully connected to {}", url);

    Ok(ws_stream.split())
}

pub type ConnSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type ConnStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn send_ws_message(sink: &mut ConnSink, msg: Message) -> Result<(), ConnectorError> {
    sink.send(msg)
        .await
        .map_err(|_| ConnectorError::WebsocketError(WebsocketError::SendMessageFailed))
}

pub async fn websocket_event_loop<F>(
    mut write: ConnSink,
    mut read: ConnStream,
    mut process_message: F,
) -> Result<(), ConnectorError>
where
    F: FnMut(&str) -> Result<(), ConnectorError>,
{
    loop {
        tokio::select! {
            msg = read.next() => {
                // println!("{:?}", msg);
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        if let Err(err) = process_message(&txt) {
                            eprintln!("Failed to process message: {:?}, error: {:?}", txt, err);
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        // Отправляем Pong с тем же payload
                        if let Err(e) = write.send(Message::Pong(payload)).await {
                            eprintln!("Failed to send Pong: {:?}", e);
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                    }
                    Some(Ok(msg)) => {
                        eprintln!("Ignoring non-text message: {:?}", msg);
                    }
                    Some(Err(err)) => {
                        eprintln!("WebSocket read error: {:?}", err);
                    }
                    None => {
                        eprintln!("WebSocket closed");
                        break; // reconnect можно делать извне
                    }
                }
            },
            _ = sleep(Duration::from_secs(20)) => {
                if let Err(e) = write.send(Message::Ping(vec![])).await {
                    eprintln!("Ping error: {:?}", e);
                }
            }
        }
    }

    Ok(())
}

pub fn parse_json<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, ConnectorError> {
    let result = serde_json::from_str::<T>(s);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(ParsingError::SerdeParseError(e))?,
    }
}

pub fn parse_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, ConnectorError> {
    let result = serde_json::from_value::<T>(value);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(ParsingError::SerdeParseError(e))?,
    }
}

pub fn parse_number(s: &str) -> Result<f64, ParsingError> {
    let result = serde_json::from_str::<f64>(s);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(ParsingError::ConvertingError(format!("{}", e))),
    }
}

pub fn parse_timestamp(s: &str) -> Result<TimestampMS, ParsingError> {
    s.parse::<TimestampMS>()
        .map_err(|e| ParsingError::ConvertingError(format!("{}", e)))
}

pub fn parse_timestamp_from_date_string(s: &str) -> Result<TimestampMS, ParsingError> {
    let dt = DateTime::parse_from_rfc3339(s)
        .map_err(|e| ParsingError::ConvertingError(format!("Failed to parse datetime: {}", e)))?;
    let timestamp_ms = dt.with_timezone(&Utc).timestamp_millis();
    Ok(timestamp_ms as TimestampMS)
}