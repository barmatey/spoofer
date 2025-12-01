use crate::connector::errors::ParsingError::{ConvertingError, MessageParsingError, SerdeError};
use crate::connector::errors::{Error, ParsingError};
use crate::shared::TimestampMS;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

pub fn get_serde_value(raw: &str) -> Result<Value, Error> {
    let result = serde_json::from_str::<Value>(raw);
    match result { 
        Ok(r) => Ok(r),
        Err(err) => Err(SerdeError(err))?,
    }
}

pub fn get_serde_object(raw: &str) -> Result<Map<String, Value>, Error> {
    let v = get_serde_value(raw)?;

    match v.as_object() {
        Some(obj) => Ok(obj.to_owned()),
        None => Err(MessageParsingError("JSON is not an object".to_string()))?,
    }
}

pub fn parse_json<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, Error> {
    let result = serde_json::from_str::<T>(s);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(SerdeError(e))?,
    }
}

pub fn parse_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, Error> {
    let result = serde_json::from_value::<T>(value);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(SerdeError(e))?,
    }
}

pub fn parse_number(s: &str) -> Result<f64, ParsingError> {
    let result = serde_json::from_str::<f64>(s);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(SerdeError(e)),
    }
}

pub fn parse_timestamp(s: &str) -> Result<TimestampMS, ParsingError> {
    s.parse::<TimestampMS>()
        .map_err(|e| ConvertingError(format!("{}", e)))
}

pub fn parse_timestamp_from_date_string(s: &str) -> Result<TimestampMS, ParsingError> {
    let dt = DateTime::parse_from_rfc3339(s)
        .map_err(|e| ConvertingError(format!("Failed to parse datetime: {}", e)))?;
    let timestamp_ms = dt.with_timezone(&Utc).timestamp_millis();
    Ok(timestamp_ms as TimestampMS)
}
