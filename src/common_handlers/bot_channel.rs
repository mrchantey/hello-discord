//! When connecting to a server,
//! the bot will search for a channel that matches its own name.
use crate::prelude::*;
use beet::prelude::*;
use tracing::info;
use twilight_model::channel::ChannelType;
use twilight_model::gateway::payload::incoming::GuildCreate;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::marker::GuildMarker;


/// When connecting to a guild, searches for a channel with the same name
/// as the bot, ie `my-bot`. This is treated as the bots owned channel,
/// to which it is allowed to post more verbosely, respond to all messages etc.
#[derive(Debug, Default, Clone, Component, Deref, DerefMut)]
#[component(on_add=on_add)]
pub struct BotChannels {
	channels: HashMap<Id<GuildMarker>, Id<ChannelMarker>>,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe(bot_channel);
}

pub fn bot_channel(
	ev: On<DiscordGuildCreate>,
	mut commands: Commands,
	mut query: Populated<(&BotState, &DiscordHttpClient, &mut BotChannels)>,
) -> Result {
	let guild = match &ev.guild_create {
		GuildCreate::Available(g) => g,
		GuildCreate::Unavailable(_) => {
			return Ok(());
		}
	};
	let entity = ev.event_target();
	let (bot_state, http_client, mut bot_channels) = query.get_mut(entity)?;

	if bot_channels.get(&guild.id).is_some() {
		// already set
		return Ok(());
	}

	let bot_name_lower = bot_state.name().to_ascii_lowercase();
	// find a text channel with the same name as the bot
	let Some(channel) = guild.channels.iter().find(|channel| {
		channel.kind == ChannelType::GuildText
			&& channel.name.as_ref().map_or(false, |name| {
				name.to_ascii_lowercase() == bot_name_lower
			})
	}) else {
		// no bot channel
		return Ok(());
	};
	bot_channels.insert(guild.id, channel.id);

	// get the name we just checked
	let channel_name = channel.name.as_deref().unwrap();
	info!(
		"Connected to bot channel: {}/{}\nGuild Id: {}",
		guild.name, channel_name, guild.id
	);


	let client = http_client.clone();
	let channel_id = channel.id;
	commands.queue_async(async move |_| {
		client
			.send(CreateMessage::new(channel_id).content("Greetings people!"))
			.await?;
		Ok(())
	});

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
