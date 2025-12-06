mod connector;
mod connector_binance;
mod errors;
mod connector_kraken;
mod builder;
mod config;

mod services;

pub use connector::{Connector, Event};
pub(crate) use connector_binance::{BinanceConnector};
pub(crate) use connector_kraken::{KrakenConnector};
pub use builder::{StreamConnector, Exchange};