use crate::connector::errors::ConnectorError;
use crate::connector::Connector;

pub struct Builder<T: Connector> {
    _marked: T,
}

impl<T: Connector> Builder<T> {
    pub fn build(&self) -> Result<T, ConnectorError> {
        Err(ConnectorError::BuilderError("failed".to_string()))
    }
}
