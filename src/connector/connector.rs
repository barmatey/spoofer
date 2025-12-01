use async_stream::stream;
use crate::connector::errors::ConnectorError;
use crate::level2::LevelUpdated;
use crate::trade::TradeEvent;
use futures::Stream;
use futures_util::StreamExt;
use crate::connector::services::websocket::{websocket_stream, Connection};

#[derive(Debug)]
pub enum Event {
    Trade(TradeEvent),
    LevelUpdate(LevelUpdated),
}

pub trait Connector {
    async fn connect(&self) -> Result<Connection, ConnectorError>;
    fn on_message(&self, msg: &str) -> Result<Vec<Event>, ConnectorError>;
    fn on_processing_error(&self, err: &ConnectorError) {
        println!("{:?}", err);
    }

    async fn stream(&self) -> Result<impl Stream<Item = Event>, ConnectorError>{
        let (write, read) = self.connect().await?;
        let ws = websocket_stream(write, read);

        let this = self;
        let s = stream! {
            futures_util::pin_mut!(ws);

            while let Some(msg) = ws.next().await {
                match msg {
                    Ok(txt) => {
                        match this.on_message(&txt) {
                            Ok(events) => {
                                for ev in events {
                                    yield ev;
                                }
                            }
                            Err(err) => {
                                self.on_processing_error(&err);
                                continue;
                            }
                        }
                    }
                    Err(err) => {
                        self.on_processing_error(&err);
                        continue;
                    }
                }
            }
        };
        Ok(s)
    }
}
