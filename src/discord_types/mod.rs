//! Discord helpers — custom types and extension traits layered on top of
//! `twilight-model`.
//!
//! This module no longer re-exports anything from `twilight-model` directly.
//! Callers that need twilight types should import them from `twilight_model`
//! themselves.  Everything defined *in this crate* (custom types, builders,
//! extension traits) is re-exported here so that
//! `use crate::discord_types::*;` (or `use crate::prelude::*;`) brings it
//! all in.

/// Custom types that don't exist in `twilight-model` (outbound message body,
/// rate-limit info, etc.)
mod custom;
pub use custom::*;
mod events;
pub use events::*;
mod ext;
pub use ext::*;
mod discord_query;
pub use discord_query::*;
