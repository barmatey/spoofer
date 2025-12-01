use crate::connector::errors::Error::InternalError;
use crate::connector::errors::{Error, ParsingError, WebsocketError};
use crate::shared::Logger;
use async_stream::try_stream;
use futures::Stream;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

pub type Connection = (
    SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
);

pub async fn connect_websocket(url: &str, logger: &Logger) -> Result<Connection, Error> {
    logger.info(&format!("🔗 Connecting to WS: {}", url));

    let parsed_url = Url::parse(url).map_err(|e| Error::from(ParsingError::UrlParseError(e)))?;

    let (ws_stream, _) = connect_async(parsed_url)
        .await
        .map_err(|_| WebsocketError::ConnectionFailed)?;

    logger.info(&format!("🟢 Successfully connected to {}", url));
    Ok(ws_stream.split())
}

pub type ConnSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type ConnStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn send_ws_message(sink: &mut ConnSink, msg: Message) -> Result<(), Error> {
    sink.send(msg)
        .await
        .map_err(|_| WebsocketError::SendMessageFailed)?;
    Ok(())
}

pub fn websocket_stream(
    mut write: ConnSink,
    mut read: ConnStream,
) -> impl Stream<Item = Result<String, Error>> {
    try_stream! {
        loop {
            tokio::select! {

                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(txt))) => {
                            yield txt;
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            if let Err(e) = write.send(Message::Pong(payload)).await {
                                yield Err(InternalError(e.to_string()))?;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // ignore
                        }
                        Some(Ok(_)) => {
                            // ignore non-text
                        }
                        Some(Err(err)) => {
                                yield Err(InternalError(err.to_string()))?;
                        }
                        None => {
                            break; // socket closed
                        }
                    }
                },

                _ = sleep(Duration::from_secs(20)) => {
                    if let Err(err) = write.send(Message::Ping(vec![])).await {
                        yield Err(InternalError(err.to_string()))?;
                    }
                }
            }
        }
    }
}
