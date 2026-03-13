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
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AsyncPlugin::default(),
		))
		.add_systems(Startup, spawn_bot)
		.run();
}

/// Startup system that spawns the discord bot.
fn spawn_bot(mut commands: Commands) {
	commands
		.spawn((
			DiscordBot::default(),
			BotState::default(),
			GreetState::default(),
		))
		.observe(|ev: On<DiscordReady>, mut commands: AsyncCommands| {
			let entity = ev.event_target();
			commands.run::<_, _, ()>(async move |world| {
				let entity = world.entity(entity);
				// todo!("call handlers from here");
			});
		});
}
