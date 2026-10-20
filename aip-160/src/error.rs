//! Errors returned by the parser and the conversions.

use thiserror::Error;

/// A `Result` whose error type is [`FilterError`].
pub type Result<T> = std::result::Result<T, FilterError>;

/// An error from parsing or converting a filter.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterError {
    /// The input does not follow the AIP-160 grammar.
    #[error("invalid filter at byte {offset}: {message}")]
    Syntax {
        /// The byte offset in the input where the error was found.
        offset: usize,
        /// A developer-facing description.
        message: String,
    },

    /// The filter is nested deeper than the parser allows.
    #[error("filter is nested deeper than {limit} levels")]
    TooDeep {
        /// The maximum nesting depth.
        limit: usize,
    },

    /// The filter names a field that the target does not support.
    #[error("unknown field: {0}")]
    InvalidField(String),

    /// A value cannot be interpreted for its field.
    #[error("invalid value for field {field}: {message}")]
    InvalidValue {
        /// The field path.
        field: String,
        /// A developer-facing description.
        message: String,
    },

    /// The filter uses a construct that the target does not support.
    #[error("unsupported filter: {0}")]
    Unsupported(String),
}
