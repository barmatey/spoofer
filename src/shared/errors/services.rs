use crate::shared::errors::BaseError;
use crate::shared::errors::BaseError::{IncompatibleExchange, OutdatedError};
use crate::shared::TimestampMS;

pub fn check_timestamp(last_ts: TimestampMS, current_ts: TimestampMS) -> Result<(), BaseError> {
    if current_ts < last_ts {
        Err(OutdatedError(
            "You are trying to add an errors that earliest last one".to_string(),
        ))?;
    }
    Ok(())
}


pub fn check_ticker(left: &str, right: &str) -> Result<(), BaseError> {
    if left != right {
        Err(IncompatibleExchange(format!(
            "Tickers are different. {} != {}",
            left, right
        )))?;
    }
    Ok(())
}

pub fn check_exchange(left: &str, right: &str) -> Result<(), BaseError> {
    if left != right {
        Err(IncompatibleExchange(format!(
            "Exchanges are different. {} != {}",
            left, right
        )))?;
    }
    Ok(())
}