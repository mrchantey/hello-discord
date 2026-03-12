//! High-level entry point for the Discord bot.
//!
//! The crate is split into two layers:
//!
//! - **Always compiled:** [`discord_types`] (re-exports from `twilight-model`
//!   plus our custom types, builders, and extension traits) and [`tl_gateway`]
//!   (lightweight gateway types borrowed from `twilight-model`).
//! - **`io` feature only:** [`discord_io`] (WebSocket gateway, HTTP REST
//!   client, event handlers, and Bevy bot wiring).

#[cfg(feature = "io")]
pub mod bot;
pub mod discord_types;
#[cfg(feature = "io")]
pub mod discord_io;
pub mod tl_gateway;

pub mod prelude {
    #[cfg(feature = "io")]
    pub use crate::bot::*;
    pub use crate::discord_types::*;
    #[cfg(feature = "io")]
    pub use crate::discord_io;
    pub use crate::tl_gateway::*;
}
