mod errors;
mod services;

pub use errors::BaseError;
pub use services::{check_ticker, check_timestamp, check_exchange, check_side};
