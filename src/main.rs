//! Discord bot entry point.
//!
//! All transport details live in `discord_io/gateway` (WebSocket) and
//! `discord_io/http` (REST). This file is purely bot logic: reacting to
//! typed events.
use beet::prelude::*;
use hello_discord::common_handlers::JoinBotChannel;
use hello_discord::prelude::*;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::marker::MessageMarker;

fn main() {
	env_ext::load_dotenv();
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

			for guild in http.send(GetCurrentUserGuilds::new()).await? {
				info!("Member of guild: {} - {}", guild.name, guild.id);
			}

			http.send(CreateTypingTrigger::new(channel_id)).await?;
			send_oneshot(
				&http,
				Actor::developer(),
				r#"
You just rejoined your own discord channel after some time,
greet the user!
"#,
				channel_id,
				None,
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
	let actor_kind = if ev.message.author.bot {
		ActorKind::Agent
	} else {
		ActorKind::User
	};
	let actor = Actor::new(&ev.message.author.name, actor_kind);
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
			send_oneshot(&http, actor, &content, channel_id, Some(msg_id))
				.await?;


			Ok(())
		});


	Ok(())
}

async fn send_oneshot(
	http: &DiscordHttpClient,
	actor: Actor,
	message: &str,
	channel_id: Id<ChannelMarker>,
	message_id: Option<Id<MessageMarker>>,
) -> Result {
	let content = oneshot_model(actor, message).await?;
	for chunk in chunk_message(&content, 2000) {
		let mut message = CreateMessage::new(channel_id).content(chunk);
		if let Some(message_id) = message_id {
			message = message.reply_to(message_id);
		}
		http.send(message).await?;
	}

	Ok(())
}

fn chunk_message(input: &str, max_len: usize) -> Vec<String> {
	let mut chunks = Vec::new();
	let mut current = String::new();

	for line in input.lines() {
		if current.len() + line.len() + 1 > max_len {
			chunks.push(current);
			current = String::new();
		}
		current.push_str(line);
		current.push('\n');
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}


async fn oneshot_model(actor: Actor, message: &str) -> Result<String> {
	let mut thread_view = ThreadMut::new();
	let out = thread_view
		.insert_actor(Actor::system())
		.insert_post(include_str!("soul.md"))
		.actor_view()
		.insert_post(
			r#"
I do not have memory, my developer is too lazy to have made that yet.
So i never bother to ask follow up questions etc, no point.
			"#,
		)
		.thread_view()
		.insert_actor(actor)
		.insert_post(message)
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(
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
		.xok();

	thread_view.despawn();
	out
}
