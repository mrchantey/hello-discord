//! High-level entry point for the Discord bot.
//!
//! The crate is split into two layers:
//!
//! - **Always compiled:** [`discord_types`] (custom types, builders, and
//!   extension traits layered on top of `twilight-model`) and [`tw_gateway`]
//!   (lightweight gateway envelope and session types).
//! - **`io` feature only:** [`discord_io`] (WebSocket gateway, HTTP REST
//!   client, event handlers, and Bevy bot wiring).
//!
//! # Importing twilight types
//!
//! This crate does **not** re-export anything from `twilight-model`. Import
//! twilight types directly:
//!
//! ```ignore
//! use twilight_model::channel::message::Message;
//! use twilight_model::id::{Id, marker::ChannelMarker};
//! ```
//!
//! `use crate::prelude::*;` (or `use hello_discord::prelude::*;`) brings in
//! all types and extension traits *defined in this crate*.

#[cfg(feature = "io")]
pub mod bot;
pub mod common_handlers;
#[cfg(feature = "io")]
pub mod discord_io;
pub mod discord_types;
pub mod tw_gateway;
pub mod tw_http;

pub mod prelude {
	#[cfg(feature = "io")]
	pub use crate::bot::*;
	pub use crate::common_handlers;
	pub use crate::common_handlers::BotChannels;
	pub use crate::common_handlers::BotState;
	#[cfg(feature = "io")]
	pub use crate::discord_io::*;
	pub use crate::discord_types::CommandExt;
	pub use crate::discord_types::*;
	pub use crate::tw_gateway::*;
	pub use crate::tw_http::*;
}
