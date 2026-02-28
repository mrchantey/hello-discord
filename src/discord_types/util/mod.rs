//! Utilities for efficiently parsing and representing data from Discord's API.
//!
//! # Fork note
//!
//! Upstream twilight-model depends on `serde-value` for intermediate
//! deserialization in custom `Deserialize` impls (components, interactions,
//! modals). We replace that with [`serde_json::Value`] plus the [`ValueExt`]
//! extension trait defined here, which provides the same `.deserialize_into()`
//! API surface so the forked deserializers need minimal changes.

pub mod datetime;
pub mod hex_color;
pub mod image_hash;
pub(crate) mod mustbe;

pub use self::{datetime::Timestamp, hex_color::HexColor, image_hash::ImageHash};

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn is_false(value: &bool) -> bool {
    !value
}

// ---------------------------------------------------------------------------
// serde-value replacement
// ---------------------------------------------------------------------------

/// Extension trait on [`serde_json::Value`] that provides
/// `.deserialize_into::<T>()`, mirroring the `serde_value::Value` API so
/// that forked twilight deserializers compile with minimal changes.
///
/// # Example
///
/// ```ignore
/// use serde_json::Value;
/// use crate::types::util::ValueExt;
///
/// let v: Value = serde_json::json!(42u64);
/// let n: u64 = v.deserialize_into().unwrap();
/// assert_eq!(n, 42);
/// ```ignore
pub(crate) trait ValueExt: Sized {
    /// Consume this value and attempt to deserialize it into `T`.
    fn deserialize_into<T: serde::de::DeserializeOwned>(self) -> Result<T, serde_json::Error>;
}

impl ValueExt for serde_json::Value {
    fn deserialize_into<T: serde::de::DeserializeOwned>(self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self)
    }
}

/// Standalone function equivalent of [`ValueExt::deserialize_into`].
///
/// Useful as a function pointer in `.map(from_json_value::<T>)` calls that
/// previously used `Value::deserialize_into`.
#[allow(dead_code)]
pub(crate) fn from_json_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, serde_json::Error> {
    serde_json::from_value(value)
}
