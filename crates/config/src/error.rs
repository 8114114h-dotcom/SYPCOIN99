// error.rs — Unified error type for the config crate.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config parse error: {0}")]
    ParseError(String),

    #[error("invalid config value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("I/O error reading config file: {0}")]
    IoError(String),
}
