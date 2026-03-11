//! Discord bot entry point.
//!
//! All transport details live in `discord_io/gateway` (WebSocket) and
//! `discord_io/http` (REST). This file is purely bot logic: reacting to
//! typed events.
use beet::prelude::*;
use hello_discord::prelude::*;

fn main() {
    dotenv::dotenv().ok();
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin::default()))
        .add_systems(Startup, spawn_bot)
        .run();
}

/// Startup system that spawns the discord bot.
fn spawn_bot(mut commands: Commands) {
    commands.spawn(DiscordBot::default());
}
