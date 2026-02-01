use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("unexpected EOF")]
    Eof,
    #[error("invalid api key: {0}")]
    InvalidApiKey(u8),
    #[error("string too long")]
    StringTooLong,
}
