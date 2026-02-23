//! Error types for [`Timestamp`] parsing.
//!
//! # Fork note
//!
//! Upstream twilight-model wraps `time::error::ComponentRange` and
//! `time::error::Parse` here. Our simplified `Timestamp` is just a
//! `String` newtype, so we only need a lightweight format error.
//!
//! [`Timestamp`]: super::Timestamp

use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

/// Reason that an ISO 8601 timestamp couldn't be parsed.
#[derive(Debug)]
pub struct TimestampParseError {
    /// Type of error that occurred.
    kind: TimestampParseErrorType,
    /// Source of the error, if there is any.
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl TimestampParseError {
    /// Error caused by the datetime being of an improper format.
    pub(crate) const FORMAT: TimestampParseError = TimestampParseError {
        kind: TimestampParseErrorType::Format,
        source: None,
    };

    /// Immutable reference to the type of error that occurred.
    #[must_use = "retrieving the type has no effect if left unused"]
    pub const fn kind(&self) -> &TimestampParseErrorType {
        &self.kind
    }

    /// Consume the error, returning the source error if there is any.
    #[must_use = "consuming the error and retrieving the source has no effect if left unused"]
    pub fn into_source(self) -> Option<Box<dyn Error + Send + Sync>> {
        self.source
    }

    /// Consume the error, returning the owned error type and the source error.
    #[must_use = "consuming the error into its parts has no effect if left unused"]
    pub fn into_parts(
        self,
    ) -> (
        TimestampParseErrorType,
        Option<Box<dyn Error + Send + Sync>>,
    ) {
        (self.kind, self.source)
    }

    /// Create a new error with a [`TimestampParseErrorType::Parsing`] kind and
    /// an arbitrary source error.
    #[allow(dead_code)]
    pub(crate) fn parsing(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            kind: TimestampParseErrorType::Parsing,
            source: Some(Box::new(source)),
        }
    }

    /// Create a new error with a [`TimestampParseErrorType::Range`] kind and
    /// an arbitrary source error.
    #[allow(dead_code)]
    pub(crate) fn range(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            kind: TimestampParseErrorType::Range,
            source: Some(Box::new(source)),
        }
    }
}

impl Display for TimestampParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match &self.kind {
            TimestampParseErrorType::Format => {
                f.write_str("provided value is not in an iso 8601 format")
            }
            TimestampParseErrorType::Parsing => f.write_str("timestamp parsing failed"),
            TimestampParseErrorType::Range => {
                f.write_str("value of a field is not in an acceptable range")
            }
        }
    }
}

impl Error for TimestampParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn Error + 'static))
    }
}

/// Type of [`TimestampParseError`] that occurred.
#[derive(Debug)]
pub enum TimestampParseErrorType {
    /// Format of the input datetime is invalid.
    ///
    /// A datetime can take two forms: with microseconds and without
    /// microseconds.
    Format,
    /// Timestamp parsing failed.
    Parsing,
    /// Value of a field is not in an acceptable range.
    Range,
}

#[cfg(test)]
mod tests {
    use super::{TimestampParseError, TimestampParseErrorType};
    use std::error::Error;

    #[test]
    fn format_error_display() {
        let err = TimestampParseError::FORMAT;
        assert_eq!(
            err.to_string(),
            "provided value is not in an iso 8601 format"
        );
    }

    #[test]
    fn format_error_has_no_source() {
        let err = TimestampParseError::FORMAT;
        assert!(err.source().is_none());
        assert!(err.into_source().is_none());
    }

    #[test]
    fn parsing_error_display() {
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let err = TimestampParseError::parsing(inner);
        assert_eq!(err.to_string(), "timestamp parsing failed");
    }

    #[test]
    fn into_parts_returns_kind_and_source() {
        let err = TimestampParseError::FORMAT;
        let (kind, source) = err.into_parts();
        assert!(matches!(kind, TimestampParseErrorType::Format));
        assert!(source.is_none());
    }
}
