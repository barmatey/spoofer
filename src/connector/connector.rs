use crate::connector::errors::ConnectorError;
use crate::level2::LevelUpdated;
use crate::trade::TradeEvent;
use futures::Stream;

#[derive(Debug)]
pub enum Event {
    Trade(TradeEvent),
    LevelUpdate(LevelUpdated),
}

pub trait Connector {
    async fn stream(&self) -> Result<impl Stream<Item = Event>, ConnectorError>;

    fn handle_error(&self, err: &ConnectorError) {
        println!("{:?}", err);
    }
}
