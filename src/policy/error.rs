use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("{message}")]
    InvalidSyntax { message: String },
    #[error("{message}")]
    Rejected { message: String },
}

impl Error {
    pub(crate) fn invalid_syntax(message: impl Into<String>) -> Self {
        Self::InvalidSyntax {
            message: message.into(),
        }
    }

    pub(crate) fn rejected(message: impl Into<String>) -> Self {
        Self::Rejected {
            message: message.into(),
        }
    }
}
