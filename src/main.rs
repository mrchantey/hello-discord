//! Discord bot entry point.
//!
//! All transport details live in `discord_io/gateway` (WebSocket) and
//! `discord_io/http` (REST). This file is purely bot logic: reacting to
//! typed events.
use beet::prelude::*;
use hello_discord::common_handlers::JoinBotChannel;
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
		.observe(on_join_bot_channel)
		.observe(on_direct_message);
}

fn on_join_bot_channel(ev: On<JoinBotChannel>, mut commands: Commands) {
	let channel_id = ev.channel;
	// ?	let
	commands
		.entity(ev.event_target())
		.queue_async(async move |entity| {
			let http = entity.get_cloned::<DiscordHttpClient>().await?;
			http.send(CreateTypingTrigger::new(channel_id)).await?;
			let greetings_message = oneshot_model(
				r#"
You just rejoined your own discord channel after some time,
greet the users!
"#,
			)
			.await?;
			http.send(
				CreateMessage::new(channel_id).content(greetings_message),
			)
			.await?;
			Ok(())
		});
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
			http.send(CreateTypingTrigger::new(channel_id)).await?;
			// http.send(CreateReaction::new(channel_id, msg_id, "👍"))
			// 	.await?;
			// let text = format!("You sent me a DM with content: {}", content);
			let text = oneshot_model(&content).await?;

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


async fn oneshot_model(message: &str) -> Result<String> {
	ThreadMut::new()
		.insert_actor(Actor::system())
		.insert_post(include_str!("soul.md"))
		.actor_view()
		.insert_post(
			r#"
I do not have memory, my developer is too lazy
for now this input and my response is all there is,
all that i remember all that i ever will
			"#,
		)
		.thread_view()
		.insert_actor(Actor::human())
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
