use std::{env, path::PathBuf};

use super::ConfigError;

const DEFAULT_CONFIG_PATH: &str = "thy-squeal.yaml";

pub(crate) fn path_from_args() -> Result<PathBuf, ConfigError> {
    let mut args = env::args().skip(1);
    let mut config_path = PathBuf::from(DEFAULT_CONFIG_PATH);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                config_path = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or(ConfigError::MissingConfigArgument)?;
            }
            other => return Err(ConfigError::UnknownArgument(other.to_owned())),
        }
    }

    Ok(config_path)
}
