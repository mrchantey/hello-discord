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
#[cfg(feature = "io")]
pub mod discord_io;
pub mod discord_types;
pub mod tl_gateway;
pub mod tw_http;

pub mod prelude {
    #[cfg(feature = "io")]
    pub use crate::bot::*;
    #[cfg(feature = "io")]
    pub use crate::discord_io;
    pub use crate::discord_types::*;
    pub use crate::tl_gateway::*;
}
