//! High-level entry point for the Discord bot.
//!
//! The crate is split into two layers:
//!
//! - **Always compiled:** [`discord_helpers`] (re-exports from `twilight-model`
//!   plus our custom types, builders, and extension traits).
//! - **`io` feature only:** [`discord_io`] (WebSocket gateway, HTTP REST
//!   client, event handlers, and Bevy bot wiring).

#[cfg(feature = "io")]
pub mod bot;
pub mod discord_helpers;
#[cfg(feature = "io")]
pub mod discord_io;

pub mod prelude {
    #[cfg(feature = "io")]
    pub use crate::bot::*;
    pub use crate::discord_helpers::*;
    #[cfg(feature = "io")]
    pub use crate::discord_io;
}
