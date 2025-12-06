use crate::connector::errors::Error;
use crate::connector::services::websocket::{websocket_stream, Connection};
use crate::level2::LevelUpdated;
use crate::trade::TradeEvent;
use async_stream::stream;
use futures::Stream;
use futures_util::StreamExt;
use std::pin::Pin;
use crossbeam::queue::SegQueue;
use crate::shared::logger::Logger;

#[derive(Debug, Clone)]
pub enum Event {
    Trade(TradeEvent),
    LevelUpdate(LevelUpdated),
}

pub type StreamBuffer = SegQueue<Event>;
pub type EventStream = Pin<Box<dyn Stream<Item = Event> + Send + Sync>>;


pub trait Connector: Send + Sync{
    async fn stream(self) -> Result<EventStream, Error>;
}

pub(crate) trait ConnectorInternal: Send + Sync {
    async fn connect(&self) -> Result<Connection, Error>;

    fn on_message(&self, msg: &str, buffer: &StreamBuffer) -> Result<(), Error>;

    fn on_error(&self, err: &Error);

    fn logger(&self) ->  &Logger;
}

impl<T: ConnectorInternal + 'static> Connector for T {
    async fn stream(self) -> Result<EventStream, Error> {
        let (write, read) = self.connect().await?;
        let ws = websocket_stream(write, read);
        let buffer: StreamBuffer = SegQueue::new();

        // перемещаем self внутрь стрима через move
        let s = stream! {
            let this = self; // владение объектом
            futures_util::pin_mut!(ws);

            loop {
                match ws.next().await {
                    Some(Ok(txt)) => {
                        match this.on_message(&txt, &buffer) {
                            Ok(()) => {
                                while let Some(ev) = buffer.pop() {
                                    yield ev;
                                }
                            }
                            Err(err) => {
                                this.on_error(&err);
                                continue;
                            }
                        }
                    }
                    Some(Err(err)) => {
                        this.on_error(&err);
                        continue;
                    }
                    None => {
                        this.logger().warn("WebSocket closed by server");
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(s) as EventStream)
    }
}
