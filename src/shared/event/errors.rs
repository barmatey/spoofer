#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("OtherError")]
    OtherError(String),
}
