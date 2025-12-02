use crate::shared::event::EventError;
use crate::shared::event::EventError::OutdatedEvent;
use crate::shared::TimestampMS;

pub fn check_timestamp(last_ts: TimestampMS, current_ts: TimestampMS) -> Result<(), EventError> {
    if current_ts < last_ts {
        Err(OutdatedEvent(
            "You are trying to add an event that earliest last one".to_string(),
        ))?;
    }
    Ok(())
}
