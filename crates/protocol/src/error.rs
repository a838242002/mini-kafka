use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("invalid api key: {0}")]
    InvalidApiKey(u8),
    #[error("io: {0}")]
    Io(#[from] common::error::IoError),
}
