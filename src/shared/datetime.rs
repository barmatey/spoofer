use crate::shared::TimestampMS;

pub fn now_timestamp() -> TimestampMS {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as TimestampMS
}
