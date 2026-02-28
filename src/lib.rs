//! High-level entry point for the Discord bot.
//!
//! The crate is split into two layers:
//!
//! - **Always compiled:** [`discord_types`] (all Discord API types) and
//!   [`events`] (the typed `GatewayEvent` enum).
//! - **`io` feature only:** [`discord_io`] (WebSocket gateway, HTTP REST
//!   client, event handlers, and Bevy bot wiring).

pub mod discord_types;
pub mod events;

#[cfg(feature = "io")]
pub mod discord_io;

#[cfg(feature = "io")]
use beet::prelude::*;

/// Run the Discord bot.
///
/// Sets up a minimal Bevy [`App`] with async support and kicks off the
/// gateway connection + event loop in [`discord_io::bot::start`].
#[cfg(feature = "io")]
pub fn run() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin::default()))
        .add_systems(Startup, start_bot)
        .run();
}

/// Startup system that spawns the async bot task.
#[cfg(feature = "io")]
fn start_bot(mut commands: AsyncCommands) {
    commands.run_local(discord_io::bot::start);
}
