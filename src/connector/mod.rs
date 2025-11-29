mod connector;
mod connector_binance;
mod errors;
mod bitstamp_connector;

pub use connector::Connector;
pub use connector_binance::{BinanceConnector, BinanceConnectorConfig};
pub use bitstamp_connector::{BitstampConnector, BitstampConnectorConfig};