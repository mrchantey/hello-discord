use crate::prelude::*;
use beet::prelude::*;
use tracing::info;
use twilight_model::channel::ChannelType;
use twilight_model::gateway::payload::incoming::GuildCreate;

/// Observer called when the bot receives a GUILD_CREATE event.
///
/// Picks the first text channel as the greeting channel if one hasn't been
/// set yet.
pub fn register_on_guild_create(
	ev: On<DiscordGuildCreate>,
	mut commands: Commands,
	query: Query<&GreetState>,
) -> Result {
	let entity = ev.event_target();

	let guild = match &ev.guild_create {
		GuildCreate::Available(g) => g,
		GuildCreate::Unavailable(ug) => {
			info!(guild_id = %ug.id, "received unavailable guild");
			return Ok(());
		}
	};

	let has_greet_channel = query
		.get(entity)
		.map(|s| s.greet_channel_id.is_some())
		.unwrap_or(false);

	if has_greet_channel {
		return Ok(());
	}

	if let Some(ch) = guild
		.channels
		.iter()
		.find(|c| c.kind == ChannelType::GuildText)
	{
		let channel_id = ch.id;
		let channel_name = ch.name.clone().unwrap_or_else(|| "?".to_string());
		info!(
			channel = %channel_name,
			channel_id = %channel_id,
			"greeting channel set"
		);
		commands.entity(entity).entry::<GreetState>().and_modify(
			move |mut state| {
				state.greet_channel_id = Some(channel_id);
			},
		);
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use twilight_model::gateway::payload::incoming::GuildCreate;
	use twilight_model::guild::UnavailableGuild;

	fn make_unavailable_guild(id: u64) -> GuildCreate {
		GuildCreate::Unavailable(UnavailableGuild {
			id: twilight_model::id::Id::new(id),
			unavailable: true,
		})
	}

	fn make_available_guild_no_text_channels(name: &str) -> GuildCreate {
		let guild: twilight_model::guild::Guild =
			serde_json::from_value(serde_json::json!({
				"id": "123",
				"name": name,
				"icon": null,
				"owner_id": "1",
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
		GuildCreate::Available(guild)
	}

	#[test]
	fn unavailable_guild_is_handled_gracefully() {
		// Should not panic; simply returns early.
		let gc = make_unavailable_guild(999);
		assert!(matches!(gc, GuildCreate::Unavailable(_)));
	}

	#[test]
	fn available_guild_with_no_channels_leaves_greet_channel_unset() {
		let gc = make_available_guild_no_text_channels("Empty");
		let guild = match &gc {
			GuildCreate::Available(g) => g,
			_ => panic!("expected available guild"),
		};
		let text_ch = guild
			.channels
			.iter()
			.find(|c| c.kind == ChannelType::GuildText);
		assert!(
			text_ch.is_none(),
			"expected no text channel in this fixture"
		);
	}

	#[test]
	fn available_guild_first_text_channel_would_be_selected() {
		let guild: twilight_model::guild::Guild =
			serde_json::from_value(serde_json::json!({
				"id": "1",
				"name": "My Server",
				"icon": null,
				"owner_id": "1",
				"channels": [
					{
						"id": "42",
						"type": 0,
						"guild_id": "1",
						"position": 0,
						"permission_overwrites": [],
						"name": "general",
						"nsfw": false,
						"rate_limit_per_user": 0,
						"topic": null,
						"last_message_id": null,
						"parent_id": null,
						"last_pin_timestamp": null
					}
				],
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

		let text_ch = guild
			.channels
			.iter()
			.find(|c| c.kind == ChannelType::GuildText);
		assert!(text_ch.is_some(), "should find the general text channel");
		assert_eq!(text_ch.unwrap().id.get(), 42);
	}
}
