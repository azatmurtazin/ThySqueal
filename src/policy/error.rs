use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message}")]
pub(crate) struct Error {
    message: String,
}

impl Error {
    pub(crate) fn rejected(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
