//! High-level entry point for the Discord bot.
//!
//! All low-level gateway, HTTP, and event-handling details live in their
//! respective modules. This file is intentionally minimal — it wires up the
//! Bevy app and delegates to [`bot::start`] for the async event loop.

pub mod bot;
pub mod events;
pub mod gateway;
pub mod handlers;
pub mod http;
pub mod types;

use beet::prelude::*;

/// Run the Discord bot.
///
/// Sets up a minimal Bevy [`App`] with async support and kicks off the
/// gateway connection + event loop in [`bot::start`].
pub fn run() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin::default()))
        .add_systems(Startup, start_bot)
        .run();
}

/// Startup system that spawns the async bot task.
fn start_bot(mut commands: AsyncCommands) {
    commands.run_local(bot::start);
}
