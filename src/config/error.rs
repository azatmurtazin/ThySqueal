use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("could not read configuration file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse configuration file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_yml::Error,
    },
    #[error("invalid configuration file {path}: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error("--config requires a path argument")]
    MissingConfigArgument,
    #[error("unknown command line argument: {0}")]
    UnknownArgument(String),
}
