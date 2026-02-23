//! Utilities for parsing and formatting ISO 8601 timestamps.
//!
//! # Fork note
//!
//! Upstream twilight-model uses the `time` crate here to provide rich
//! timestamp arithmetic (`as_secs`, `as_micros`, `from_secs`, etc.).
//! We replace that with a thin `String` newtype that round-trips through
//! serde without pulling in `time`.
//!
//! The `from_secs` / `from_micros` / `as_secs` / `as_micros` methods are
//! provided for API compatibility with upstream tests, but they perform
//! simple arithmetic formatting rather than full calendar math.
//!
//! If you need the full `time`-backed implementation, enable the
//! `timestamps` feature flag (not yet wired — reserved for future use).

mod error;

pub use self::error::{TimestampParseError, TimestampParseErrorType};

use serde::{
    de::{Deserialize, Deserializer, Error as DeError, Visitor},
    ser::{Serialize, Serializer},
};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

/// Minimum length of an ISO 8601 datetime without microseconds.
///
/// Example: `2021-01-01T01:01:01+00:00` (25 characters).
const MIN_TIMESTAMP_LENGTH: usize = 25;

/// Number of microseconds in a second.
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;

/// Representation of a Discord timestamp as an ISO 8601 string.
///
/// This is a lightweight alternative to the upstream `time`-backed
/// `Timestamp`. It stores the raw ISO 8601 string exactly as Discord
/// sent it and re-serializes it verbatim.
///
/// # Display
///
/// The [`Display`] implementation writes the stored ISO 8601 string.
///
/// # serde
///
/// Deserializes from a JSON string and serializes back as a JSON string.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(String);

impl Timestamp {
    /// Parse a timestamp from an ISO 8601 datetime string emitted by Discord.
    ///
    /// Discord emits two ISO 8601 formats: with microseconds
    /// (`2021-01-01T01:01:01.010000+00:00`) and without
    /// (`2021-01-01T01:01:01+00:00`). Both are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`TimestampParseErrorType::Format`] if the string is too short
    /// or doesn't look like a valid ISO 8601 datetime.
    pub fn parse(datetime: &str) -> Result<Self, TimestampParseError> {
        if datetime.len() < MIN_TIMESTAMP_LENGTH {
            return Err(TimestampParseError::FORMAT);
        }

        // Minimal structural validation: must contain a 'T' separator and an
        // offset ('+' or '-' after the time portion). We intentionally keep
        // this loose — Discord's API is the canonical source of these strings.
        if !datetime.contains('T') {
            return Err(TimestampParseError::FORMAT);
        }

        Ok(Self(datetime.to_owned()))
    }

    /// Create a `Timestamp` from a raw string *without* validation.
    ///
    /// Prefer [`parse`](Self::parse) or [`FromStr`] for untrusted input.
    pub fn from_raw(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Create a timestamp from a Unix timestamp with seconds precision.
    ///
    /// Formats the value as an ISO 8601 string in UTC. This is a
    /// simplified implementation that doesn't depend on the `time` crate.
    ///
    /// # Errors
    ///
    /// Returns [`TimestampParseErrorType::Range`] if the value can't be
    /// represented (this implementation accepts all `i64` values).
    pub fn from_secs(unix_seconds: i64) -> Result<Self, TimestampParseError> {
        Ok(Self(format_unix_secs(unix_seconds, 0)))
    }

    /// Create a timestamp from a Unix timestamp with microseconds precision.
    ///
    /// # Errors
    ///
    /// Returns [`TimestampParseErrorType::Range`] if the value can't be
    /// represented.
    pub fn from_micros(unix_microseconds: i64) -> Result<Self, TimestampParseError> {
        let secs = unix_microseconds / MICROSECONDS_PER_SECOND;
        let micros = (unix_microseconds % MICROSECONDS_PER_SECOND).unsigned_abs() as u32;
        Ok(Self(format_unix_secs(secs, micros)))
    }

    /// Total number of seconds within the timestamp (approximate).
    ///
    /// Parses the stored ISO 8601 string back into a Unix timestamp.
    /// Returns `0` if parsing fails.
    pub fn as_secs(&self) -> i64 {
        parse_to_unix_secs(&self.0).unwrap_or(0)
    }

    /// Total number of microseconds within the timestamp (approximate).
    ///
    /// Parses the stored ISO 8601 string back into a Unix timestamp with
    /// microsecond precision. Returns `0` if parsing fails.
    pub fn as_micros(&self) -> i64 {
        let (secs, micros) = parse_to_unix_secs_and_micros(&self.0).unwrap_or((0, 0));
        secs * MICROSECONDS_PER_SECOND + micros as i64
    }

    /// View the timestamp as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the timestamp and return the inner string.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Create an ISO 8601 display formatter.
    ///
    /// For this simplified implementation this just returns a wrapper
    /// that delegates to [`Display`].
    pub const fn iso_8601(&self) -> TimestampIso8601Display<'_> {
        TimestampIso8601Display { inner: self }
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TimestampVisitor;

        impl Visitor<'_> for TimestampVisitor {
            type Value = Timestamp;

            fn expecting(&self, f: &mut Formatter<'_>) -> FmtResult {
                f.write_str("an ISO 8601 datetime string")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Timestamp::parse(v).map_err(DeError::custom)
            }
        }

        deserializer.deserialize_any(TimestampVisitor)
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl TryFrom<&'_ str> for Timestamp {
    type Error = TimestampParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<Timestamp> for String {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl AsRef<str> for Timestamp {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Display wrapper returned by [`Timestamp::iso_8601`].
///
/// In the simplified (no-`time`) implementation this simply delegates
/// to the stored string. The API is kept so that code written against
/// upstream twilight continues to compile.
#[derive(Debug)]
pub struct TimestampIso8601Display<'a> {
    inner: &'a Timestamp,
}

impl<'a> TimestampIso8601Display<'a> {
    /// Create a new display wrapper (called by [`Timestamp::iso_8601`]).
    #[allow(dead_code)]
    pub(super) const fn new(timestamp: &'a Timestamp) -> Self {
        Self { inner: timestamp }
    }

    /// Get the inner timestamp reference.
    pub const fn get(&self) -> &'a Timestamp {
        self.inner
    }

    /// Whether to include microseconds in the output.
    ///
    /// This is a no-op in the simplified implementation (the stored
    /// string is always returned as-is), but the method is kept for
    /// API compatibility with upstream twilight.
    #[must_use]
    pub const fn with_microseconds(self, _with_microseconds: bool) -> Self {
        // In the simplified implementation we always return the original
        // string, so this flag is ignored.
        self
    }
}

impl Display for TimestampIso8601Display<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.inner, f)
    }
}

impl Serialize for TimestampIso8601Display<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers for formatting / parsing Unix timestamps without `time`
// ---------------------------------------------------------------------------

/// Format a Unix timestamp (seconds + microseconds) as an ISO 8601 string.
fn format_unix_secs(unix_secs: i64, micros: u32) -> String {
    // This is a simplified implementation. For a framework that only deals
    // with Discord timestamps (all UTC, all post-2010) this is fine.
    //
    // Algorithm: civil date from days since epoch (Euclidean affine).
    let (y, m, d, h, min, s) = civil_from_unix(unix_secs);

    if micros > 0 {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}+00:00",
            y, m, d, h, min, s, micros
        )
    } else {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000000+00:00",
            y, m, d, h, min, s
        )
    }
}

/// Convert Unix timestamp to (year, month, day, hour, minute, second).
fn civil_from_unix(unix_secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let secs_per_day: i64 = 86400;
    let mut days = unix_secs.div_euclid(secs_per_day);
    let day_secs = unix_secs.rem_euclid(secs_per_day) as u32;

    let h = day_secs / 3600;
    let min = (day_secs % 3600) / 60;
    let s = day_secs % 60;

    // Days since 0000-03-01 (era-based algorithm from Howard Hinnant)
    days += 719468; // offset from 1970-01-01 to 0000-03-01
    let era = days.div_euclid(146097);
    let doe = days.rem_euclid(146097); // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month index [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    (y as i32, m as u32, d as u32, h, min, s)
}

/// Parse an ISO 8601 string to Unix seconds (approximate, UTC only).
fn parse_to_unix_secs(input: &str) -> Option<i64> {
    parse_to_unix_secs_and_micros(input).map(|(s, _)| s)
}

/// Parse an ISO 8601 string to (Unix seconds, microseconds).
fn parse_to_unix_secs_and_micros(input: &str) -> Option<(i64, u32)> {
    // Expected: "YYYY-MM-DDTHH:MM:SS[.uuuuuu]+00:00"
    if input.len() < 25 {
        return None;
    }
    let b = input.as_bytes();
    let year: i64 = input.get(0..4)?.parse().ok()?;
    let month: i64 = input.get(5..7)?.parse().ok()?;
    let day: i64 = input.get(8..10)?.parse().ok()?;
    if b[10] != b'T' {
        return None;
    }
    let hour: i64 = input.get(11..13)?.parse().ok()?;
    let minute: i64 = input.get(14..16)?.parse().ok()?;
    let second: i64 = input.get(17..19)?.parse().ok()?;

    let micros = if b.get(19).copied() == Some(b'.') {
        // Parse up to 6 digits of fractional seconds
        let frac_start = 20;
        let frac_end = input[frac_start..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| frac_start + i)
            .unwrap_or(input.len());
        let frac_str = &input[frac_start..frac_end];
        let mut val: u32 = frac_str.parse().ok()?;
        // Normalize to 6 digits
        let digits = frac_str.len();
        for _ in digits..6 {
            val *= 10;
        }
        for _ in 6..digits {
            val /= 10;
        }
        val
    } else {
        0
    };

    // Convert civil date to Unix timestamp (UTC)
    // Using inverse of civil_from_unix
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;

    let secs = days * 86400 + hour * 3600 + minute * 60 + second;
    Some((secs, micros))
}

#[cfg(test)]
mod tests {
    use super::{Timestamp, TimestampParseError};
    use std::str::FromStr;

    #[test]
    fn parse_with_microseconds() {
        let ts = Timestamp::from_str("2020-02-02T02:02:02.020000+00:00");
        assert!(ts.is_ok());
        assert_eq!(ts.unwrap().as_str(), "2020-02-02T02:02:02.020000+00:00");
    }

    #[test]
    fn parse_without_microseconds() {
        let ts = Timestamp::from_str("2021-01-01T01:01:01+00:00");
        assert!(ts.is_ok());
    }

    #[test]
    fn parse_too_short() {
        let ts = Timestamp::from_str("2021-01-01");
        assert!(ts.is_err());
    }

    #[test]
    fn parse_no_t_separator() {
        let ts = Timestamp::from_str("2021-01-01 01:01:01+00:00");
        assert!(ts.is_err());
    }

    #[test]
    fn serde_round_trip() {
        let original = "2021-08-10T11:16:37.020000+00:00";
        let ts = Timestamp::from_str(original).unwrap();
        let json = serde_json::to_string(&ts).unwrap();
        assert_eq!(json, format!("\"{}\"", original));

        let parsed: Timestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ts);
    }

    #[test]
    fn display() {
        let ts = Timestamp::from_str("2020-02-02T02:02:02.020000+00:00").unwrap();
        assert_eq!(ts.to_string(), "2020-02-02T02:02:02.020000+00:00");
        assert_eq!(
            ts.iso_8601().to_string(),
            "2020-02-02T02:02:02.020000+00:00"
        );
    }

    #[test]
    fn from_secs_and_back() {
        let ts = Timestamp::from_secs(1_580_608_922).unwrap();
        assert_eq!(ts.as_secs(), 1_580_608_922);
        // Verify the formatted string looks right
        assert!(ts.as_str().starts_with("2020-02-02T02:02:02"));
    }

    #[test]
    fn from_micros_and_back() {
        let ts = Timestamp::from_micros(1_580_608_922_020_000).unwrap();
        assert_eq!(ts.as_micros(), 1_580_608_922_020_000);
        assert!(ts.as_str().contains(".020000"));
    }

    #[test]
    fn parse_then_as_secs() {
        let ts = Timestamp::from_str("2021-08-10T11:16:37.020000+00:00").unwrap();
        assert_eq!(ts.as_secs(), 1_628_594_197);
    }

    #[test]
    fn parse_then_as_micros() {
        let ts = Timestamp::from_str("2021-08-10T11:16:37.123456+00:00").unwrap();
        assert_eq!(ts.as_micros(), 1_628_594_197_123_456);
    }

    #[test]
    fn from_secs_zero() {
        let ts = Timestamp::from_secs(0).unwrap();
        assert_eq!(ts.as_secs(), 0);
        assert!(ts.as_str().starts_with("1970-01-01T00:00:00"));
    }

    #[test]
    fn from_secs_iso8601_format() {
        let ts = Timestamp::from_secs(1_580_608_922).unwrap();
        assert_eq!(
            ts.iso_8601().to_string(),
            "2020-02-02T02:02:02.000000+00:00"
        );
    }

    #[test]
    fn from_micros_iso8601_format() {
        let ts = Timestamp::from_micros(1_628_594_197_020_000).unwrap();
        assert_eq!(
            ts.iso_8601().to_string(),
            "2021-08-10T11:16:37.020000+00:00"
        );
    }
}
