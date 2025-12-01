use crate::connector::errors::ConnectorError;
use crate::connector::services::websocket::{websocket_stream, Connection};
use crate::level2::LevelUpdated;
use crate::trade::TradeEvent;
use async_stream::stream;
use futures::Stream;
use futures_util::StreamExt;
use std::collections::VecDeque;

#[derive(Debug)]
pub enum Event {
    Trade(TradeEvent),
    LevelUpdate(LevelUpdated),
}

pub type StreamBuffer = VecDeque<Event>;

pub trait Connector {
    async fn stream(&self) -> Result<impl Stream<Item = Event>, ConnectorError>;
}

pub(crate) trait ConnectorInternal {
    async fn connect(&self) -> Result<Connection, ConnectorError>;

    fn on_message(&self, msg: &str, buffer: &mut StreamBuffer) -> Result<(), ConnectorError>;

    fn on_error(&self, err: &ConnectorError);
}


impl<T: ConnectorInternal> Connector for T {
    async fn stream(&self) -> Result<impl Stream<Item = Event>, ConnectorError> {
        let (write, read) = self.connect().await?;
        let ws = websocket_stream(write, read);
        let mut buffer: StreamBuffer = VecDeque::new();

        let this = self;

        let s = stream! {
            futures_util::pin_mut!(ws);
            while let Some(msg) = ws.next().await {
                match msg {
                    Ok(txt) => {
                        match this.on_message(&txt, &mut buffer) {
                            Ok(()) => {
                                while let Some(ev) = buffer.pop_front(){
                                    yield ev;
                                }
                            }
                            Err(err) => {
                                this.on_error(&err);
                                continue;
                            }
                        }
                    }
                    Err(err) => {
                        this.on_error(&err);
                        continue;
                    }
                }
            }
        };

        Ok(s)
    }
}
