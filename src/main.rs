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
		.spawn((DiscordBot::default(), BotChannels::default()))
		.observe(common_handlers::set_bot_state)
		.observe(on_direct_message);
}


fn on_direct_message(
	ev: On<DiscordMessage>,
	mut commands: Commands,
	query: DiscordQuery,
) -> Result {
	if !query.message_info(&ev)?.is_direct_message() {
		return Ok(());
	}

	let msg_id = ev.message.id;
	let channel_id = ev.message.channel_id;
	let content = ev.message.content.clone();
	commands
		.entity(ev.event_target())
		.queue_async(async move |entity| {
			let http = entity.get_cloned::<DiscordHttpClient>().await?;

			let text = format!("You sent me a DM with content: {}", content);

			http.create_message(
				channel_id,
				&CreateMessage::new().content(text).reply_to(msg_id),
			)
			.await?;

			Ok(())
		});


	Ok(())
}
