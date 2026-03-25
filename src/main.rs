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
		.observe(common_handlers::init_bot_state)
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

			// let text = format!("You sent me a DM with content: {}", content);

			let text = completion(&content).await?;

			http.send(
				CreateMessage::new(channel_id)
					.content(text)
					.reply_to(msg_id),
			)
			.await?;

			Ok(())
		});


	Ok(())
}


async fn completion(message: &str) -> Result<String> {
	ThreadMut::new()
		.insert_actor(Actor::system())
		.insert_post(message)
		.thread_view()
		.insert_actor(Actor::agent())
		.with_streamer(
			OpenAiProvider::gpt_5_mini()?
				// OllamaProvider::qwen_3_8b()
				// disable streaming since we're aggregating
				.without_streaming(),
		)
		.send_and_collect()
		.await
		.unwrap()
		.into_iter()
		.filter(|post| post.intent().is_display())
		.xtry_map(|post| post.body_string())?
		.join("\n")
		.xok()
}
