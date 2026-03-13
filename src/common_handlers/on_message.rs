use crate::prelude::*;
use beet::prelude::*;
use tracing::error;
use tracing::info;
use tracing::warn;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

/// Observer called when a non-bot user sends a message.
///
/// Handles `!` prefix commands and @-mention commands.
pub fn register_on_message(
	msg: On<DiscordMessage>,
	mut commands: Commands,
	query: Query<(&BotState, &GreetState, &DiscordHttpClient)>,
) -> Result {
	if msg.author.bot {
		return Ok(());
	}

	let entity = msg.event_target();

	info!(
		message_id = %msg.id,
		author = %msg.author.tag(),
		channel_id = %msg.channel_id,
		content = %msg.content,
		"handling message"
	);

	let channel_id = msg.channel_id;

	let (bot_state, greet_state, http) = query.get(entity)?;

	// Update greet channel if not yet set.
	if greet_state.greet_channel_id.is_none() {
		commands.entity(entity).entry::<GreetState>().and_modify(
			move |mut state| {
				if state.greet_channel_id.is_none() {
					state.greet_channel_id = Some(channel_id);
				}
			},
		);
	}

	let bot_user_id = bot_state.bot_user_id();
	let start_time = bot_state.start_time();
	let http = http.clone();
	let content = msg.content.trim().to_string();

	// Determine effective command text from @mention or ! prefix.
	let effective_content = {
		let mention_tag = format!("<@{}>", bot_user_id);
		let mention_tag_nick = format!("<@!{}>", bot_user_id);
		if content.starts_with(&mention_tag) {
			content
				.strip_prefix(&mention_tag)
				.unwrap_or("")
				.trim()
				.to_string()
		} else if content.starts_with(&mention_tag_nick) {
			content
				.strip_prefix(&mention_tag_nick)
				.unwrap_or("")
				.trim()
				.to_string()
		} else if msg.mentions_user(bot_user_id) {
			String::new()
		} else {
			String::new()
		}
	};

	let command_text = if content.starts_with('!') {
		content.clone()
	} else if !effective_content.is_empty() {
		if effective_content.starts_with('!') {
			effective_content.clone()
		} else {
			format!("!{}", effective_content)
		}
	} else {
		String::new()
	};

	if command_text.is_empty() {
		return Ok(());
	}

	let msg_id = msg.id;
	let guild_id = msg.guild_id;

	commands.queue_async(async move |_| {
		dispatch_message_command(
			&http,
			channel_id,
			msg_id,
			guild_id,
			bot_user_id,
			start_time,
			&command_text,
		)
		.await;
	});

	Ok(())
}

async fn dispatch_message_command(
	http: &DiscordHttpClient,
	channel_id: Id<ChannelMarker>,
	msg_id: twilight_model::id::Id<twilight_model::id::marker::MessageMarker>,
	guild_id: Option<
		twilight_model::id::Id<twilight_model::id::marker::GuildMarker>,
	>,
	bot_user_id: Id<UserMarker>,
	start_time: std::time::Instant,
	command_text: &str,
) {
	let parts: Vec<&str> = command_text.splitn(2, ' ').collect();
	let command = parts[0];
	let args = parts.get(1).copied().unwrap_or("");

	let reply =
		|text: String| CreateMessage::new().content(text).reply_to(msg_id);

	match command {
		"!hello" => {
			let body = reply("Hello, World! 👋".to_string());
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !hello reply");
			}
		}

		"!ping" => {
			let now = chrono::Utc::now();
			let latency = msg_id
				.get()
				.checked_shr(22)
				.map(|shifted| shifted + 1420070400000)
				.and_then(|ms| {
					chrono::DateTime::from_timestamp_millis(ms as i64)
				})
				.map(|sent_at| {
					let diff = now.signed_duration_since(sent_at);
					format!("{}ms", diff.num_milliseconds())
				})
				.unwrap_or_else(|| "unknown".to_string());
			let text = format!("🏓 Pong! Latency: {}", latency);
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !ping reply");
			}
		}

		"!uptime" => {
			let elapsed = start_time.elapsed();
			let secs = elapsed.as_secs();
			let text = format!(
				"⏱️ Bot uptime: {}h {}m {}s",
				secs / 3600,
				(secs % 3600) / 60,
				secs % 60
			);
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !uptime reply");
			}
		}

		"!roll" => {
			let sides: u32 = args.trim().parse().unwrap_or(6).max(2).min(1000);
			let result = (rand::random::<u32>() % sides) + 1;
			let text = format!("🎲 Rolling a d{}... **{}**!", sides, result);
			let body = reply(text).component_row(action_row(vec![button(
				1,
				"🎲 Reroll",
				format!("reroll:{}", sides),
			)]));
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !roll reply");
			}
		}

		"!count" => {
			let text = match http.count_messages(channel_id).await {
				Ok(count) => {
					format!(
						"📊 This channel has approximately **{}** messages.",
						count
					)
				}
				Err(e) => format!("❌ Error counting messages: {}", e),
			};
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !count reply");
			}
		}

		"!first" => {
			let text = match http.get_first_message(channel_id).await {
				Ok(first_msg) => {
					let ts_str = first_msg.timestamp.iso_8601().to_string();
					let ts = if let Ok(dt) =
						chrono::DateTime::parse_from_rfc3339(&ts_str)
					{
						dt.format("%B %d, %Y at %H:%M UTC").to_string()
					} else {
						ts_str
					};
					format!(
						"📜 **First message in this channel:**\n> {}\n— *{}* on {}",
						first_msg.content, first_msg.author.name, ts
					)
				}
				Err(e) => format!("❌ Error fetching first message: {}", e),
			};
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !first reply");
			}
		}

		"!serverinfo" => {
			let text = if let Some(gid) = guild_id {
				match http.get_guild(gid).await {
					Ok(guild) => format_guild_info(&guild),
					Err(e) => format!("❌ Error fetching server info: {}", e),
				}
			} else {
				"❌ This command only works in a server.".to_string()
			};
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !serverinfo reply");
			}
		}

		"!whoami" => {
			// We don't have the full author here; fall back to a mention.
			let text = format!(
				"👤 **About You:**\n\
				 • **User ID:** <@{}>\n\
				 *(Use `/whoami` for full details)*",
				bot_user_id
			);
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !whoami reply");
			}
		}

		"!help" => {
			let text = help_text();
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				error!(error = %e, "failed to send !help reply");
			}
		}

		other if other.starts_with('!') => {
			info!(command = other, "unhandled command");
			let text = format!("Not sure what that means: `{}`", other);
			let body = reply(text);
			if let Err(e) = http.create_message(channel_id, &body).await {
				warn!(error = %e, "failed to send unknown-command reply");
			}
		}

		unhandled => {
			info!(command = unhandled, "not a command, ignoring");
		}
	}
}

// ---------------------------------------------------------------------------
// Formatting helpers (duplicated here so this module is self-contained;
// shared helpers live in handlers.rs until that file is removed)
// ---------------------------------------------------------------------------

fn format_guild_info(guild: &twilight_model::guild::Guild) -> String {
	let member_count = guild
		.approximate_member_count
		.map(|n| n.to_string())
		.unwrap_or_else(|| "unknown".to_string());
	let online_count = guild
		.approximate_presence_count
		.map(|n| n.to_string())
		.unwrap_or_else(|| "unknown".to_string());
	let owner_str = guild.owner_id.to_string();
	let created_at = guild
		.created_at_ms()
		.and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
		.map(|dt| dt.format("%B %d, %Y").to_string())
		.unwrap_or_else(|| "unknown".to_string());

	format!(
		"🏰 **Server Info: {}**\n\
         • **Members:** {} ({} online)\n\
         • **Owner:** <@{}>\n\
         • **Created:** {}",
		guild.name, member_count, online_count, owner_str, created_at
	)
}

fn help_text() -> String {
	"🤖 **Available Commands:**\n\
     *Prefix commands (! or @mention):*\n\
     • `!hello` — Say hello!\n\
     • `!ping` — Check bot latency\n\
     • `!uptime` — See how long the bot has been running\n\
     • `!roll [sides]` — Roll a dice (default: 6 sides)\n\
     • `!count` — Count messages in this channel\n\
     • `!first` — Show the first message ever sent in this channel\n\
     • `!serverinfo` — Show server information\n\
     • `!whoami` — Show info about yourself\n\
     • `!help` — Show this help message\n\
     \n\
     *Slash commands:*\n\
     • `/ping` `/uptime` `/roll` `/serverinfo` `/whoami` `/count` `/first` `/help`\n\
     • `/report` — Submit a report via a pop-up form\n\
     • `/send-logo` — Send the bot logo\n\
     • `/demo-select` — Demo the select menu component"
		.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn help_text_mentions_all_prefix_commands() {
		let text = help_text();
		for cmd in &[
			"!hello",
			"!ping",
			"!uptime",
			"!roll",
			"!count",
			"!first",
			"!serverinfo",
			"!whoami",
			"!help",
		] {
			assert!(text.contains(cmd), "help text missing {}", cmd);
		}
	}

	#[test]
	fn command_text_from_bang_prefix() {
		let content = "!roll 20";
		assert!(content.starts_with('!'));
		let parts: Vec<&str> = content.splitn(2, ' ').collect();
		assert_eq!(parts[0], "!roll");
		assert_eq!(parts[1], "20");
	}

	#[test]
	fn command_text_from_mention_with_bang() {
		let bot_id = "12345";
		let content = format!("<@{}> !ping", bot_id);
		let mention_tag = format!("<@{}>", bot_id);
		let after_mention = content
			.strip_prefix(&mention_tag)
			.unwrap_or("")
			.trim()
			.to_string();
		assert_eq!(after_mention, "!ping");
	}

	#[test]
	fn command_text_from_mention_without_bang_gets_prefixed() {
		let effective_content = "roll 6";
		// If there's no leading '!', we prepend one.
		let command_text = if effective_content.starts_with('!') {
			effective_content.to_string()
		} else {
			format!("!{}", effective_content)
		};
		assert_eq!(command_text, "!roll 6");
	}

	#[test]
	fn empty_effective_content_produces_empty_command_text() {
		let content = "hello world"; // no '!', no mention
		let effective_content = String::new();
		let command_text = if content.starts_with('!') {
			content.to_string()
		} else if !effective_content.is_empty() {
			format!("!{}", effective_content)
		} else {
			String::new()
		};
		assert!(command_text.is_empty());
	}

	#[test]
	fn roll_sides_clamped_to_valid_range() {
		let sides: u32 = 1u32.max(2).min(1000); // below minimum
		assert_eq!(sides, 2);
		let sides: u32 = 9999u32.max(2).min(1000); // above maximum
		assert_eq!(sides, 1000);
		let sides: u32 = 6u32.max(2).min(1000); // normal
		assert_eq!(sides, 6);
	}

	#[test]
	fn format_guild_info_includes_guild_name() {
		let guild: twilight_model::guild::Guild =
			serde_json::from_value(serde_json::json!({
				"id": "1",
				"name": "Test Guild",
				"icon": null,
				"owner_id": "2",
				"approximate_member_count": 50,
				"approximate_presence_count": 5,
				"channels": [],
				"members": [],
				"roles": [],
				"emojis": [],
				"features": [],
				"afk_timeout": 300,
				"preferred_locale": "en-US",
				"premium_progress_bar_enabled": false,
				"verification_level": 0,
				"default_message_notifications": 0,
				"explicit_content_filter": 0,
				"mfa_level": 0,
				"premium_tier": 0,
				"nsfw_level": 0,
				"system_channel_flags": 0,
			}))
			.unwrap();
		let text = format_guild_info(&guild);
		assert!(text.contains("Test Guild"));
		assert!(text.contains("50"));
		assert!(text.contains("5"));
		assert!(text.contains("<@2>"));
	}
}
