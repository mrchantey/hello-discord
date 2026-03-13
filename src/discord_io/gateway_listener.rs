//! Core bot infrastructure: Bevy Resources, gateway bridge, and async event loop.
//!
//! This module owns the "engine" of the bot — connecting to Discord's gateway,
//! polling events, and dispatching them to handler functions. All mutable state
//! lives in Bevy [`Resource`]s accessed through [`AsyncWorld`], so no manual
//! mutexes are needed in the bot layer.

use crate::prelude::*;
use beet::prelude::*;
use std::collections::HashSet;
use std::time::Instant;
use twilight_model::gateway::event::DispatchEvent;
use twilight_model::gateway::event::GatewayEvent;
use twilight_model::gateway::Intents;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

/// Core bot identity and lifecycle state.
#[derive(Component)]
pub struct BotState {
	/// The bot's own user ID (set on READY).
	bot_user_id: Id<UserMarker>,
	/// The application ID (set on READY).
	application_id: Id<ApplicationMarker>,
	/// Timestamp of when the bot started.
	start_time: Instant,
}

impl BotState {
	pub fn new(
		bot_user_id: Id<UserMarker>,
		application_id: Id<ApplicationMarker>,
	) -> Self {
		Self {
			bot_user_id,
			application_id,
			start_time: Instant::now(),
		}
	}
	pub fn bot_user_id(&self) -> Id<UserMarker> { self.bot_user_id }
	pub fn application_id(&self) -> Id<ApplicationMarker> {
		self.application_id
	}
	pub fn start_time(&self) -> Instant { self.start_time }
}
/// State for the "greet users who come online" feature.
#[derive(Component, Default)]
pub struct GreetState {
	/// Channel to send greeting messages in.
	pub greet_channel_id: Option<Id<ChannelMarker>>,
	/// Users we've already greeted this session (to avoid spamming).
	pub greeted_users: HashSet<Id<UserMarker>>,
}

// ---------------------------------------------------------------------------
// Gateway intents
// ---------------------------------------------------------------------------

/// Build the gateway intents using strongly-typed [`Intents`] bitflags.
fn gateway_intents() -> Intents {
	Intents::GUILDS
		| Intents::GUILD_MEMBERS
		| Intents::GUILD_PRESENCES
		| Intents::GUILD_MESSAGES
		| Intents::MESSAGE_CONTENT
}

// ---------------------------------------------------------------------------
// Bot entry point
// ---------------------------------------------------------------------------

/// Async entry point for the bot.
///
/// Called from a Bevy startup system via [`AsyncCommands::run_local`].
/// Initialises Resources, connects to the Discord gateway, and runs the
/// main event loop — dispatching each event to the appropriate handler
/// in [`crate::handlers`].
pub async fn start_gateway_listener(entity: AsyncEntity) -> Result {
	let token = entity
		.get::<DiscordBot, _>(|bot| bot.token().to_string())
		.await?;

	// Create the HTTP client (cheap to clone — Arc internals).
	let http = DiscordHttpClient::new(&token);
	entity.insert_then(http.clone()).await;

	// Insert state into the Bevy world as Resources.

	// Connect to the Discord gateway.
	let gw = GatewayConfig {
		token,
		intents: gateway_intents(),
		shard: None, // single-shard
	}
	.connect()
	.await
	.map_err(|e| {
		error!(error = %e, "failed to start gateway");
		e
	})?;

	info!("gateway connected, entering event loop");

	// ----- Main event loop -----
	while let Ok(event) = gw.events.recv().await {
		trace!("Event Received: {event:#?}");

		match event {
			GatewayEvent::Dispatch(_, ref dispatch) => match dispatch {
				DispatchEvent::Ready(ready) => {
					entity.trigger(DiscordReady::create(ready.clone()));
				}

				DispatchEvent::GuildCreate(guild_create) => {
					handlers::on_guild_create(&entity, guild_create).await;
				}

				DispatchEvent::PresenceUpdate(presence) => {
					handlers::on_presence_update(&entity, &http, presence)
						.await;
				}

				DispatchEvent::MessageCreate(msg) => {
					if msg.author.bot {
						continue;
					}
					handlers::on_message(&entity, &http, msg.0.clone()).await;
				}

				DispatchEvent::InteractionCreate(interaction) => {
					if let Err(e) =
						handlers::on_interaction(&entity, &http, &interaction.0)
							.await
					{
						error!(error = %e, "failed to handle interaction");
					}
				}

				other => {
					tracing::trace!(event = ?other, "unhandled dispatch event");
				}
			},

			// Heartbeat ACK — already logged at debug level in gateway module.
			GatewayEvent::HeartbeatAck => {}

			// Reconnect / InvalidSession are handled internally by the gateway driver.
			GatewayEvent::Reconnect | GatewayEvent::InvalidateSession(_) => {}

			// Heartbeat request — handled by the gateway driver.
			GatewayEvent::Heartbeat => {}

			// Hello — handled during connection setup.
			GatewayEvent::Hello(_) => {}
		}
	}

	warn!("event stream ended, bot shutting down");
	Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;

	// -- GreetState --------------------------------------------------------

	#[test]
	fn greet_state_default_is_empty() {
		let state = GreetState::default();
		assert!(state.greet_channel_id.is_none());
		assert!(state.greeted_users.is_empty());
	}

	#[test]
	fn greet_state_tracks_greeted_users() {
		let mut state = GreetState::default();
		let user_a: Id<UserMarker> = Id::new(111);
		let user_b: Id<UserMarker> = Id::new(222);
		assert!(!state.greeted_users.contains(&user_a));
		state.greeted_users.insert(user_a);
		assert!(state.greeted_users.contains(&user_a));
		assert!(!state.greeted_users.contains(&user_b));
	}

	#[test]
	fn greet_state_no_duplicate_greetings() {
		let mut state = GreetState::default();
		let user_a: Id<UserMarker> = Id::new(111);
		assert!(state.greeted_users.insert(user_a));
		// Second insert returns false — user already present.
		assert!(!state.greeted_users.insert(user_a));
		assert_eq!(state.greeted_users.len(), 1);
	}

	#[test]
	fn greet_state_channel_id_can_be_set() {
		let mut state = GreetState::default();
		let chan: Id<ChannelMarker> = Id::new(123);
		state.greet_channel_id = Some(chan);
		assert_eq!(state.greet_channel_id.map(|id| id.get()), Some(123));
	}

	// -- gateway_intents() -------------------------------------------------

	#[test]
	fn gateway_intents_includes_required_bits() {
		let intents = gateway_intents();
		assert!(intents.contains(Intents::GUILDS), "missing GUILDS");
		assert!(
			intents.contains(Intents::GUILD_MEMBERS),
			"missing GUILD_MEMBERS"
		);
		assert!(
			intents.contains(Intents::GUILD_PRESENCES),
			"missing GUILD_PRESENCES"
		);
		assert!(
			intents.contains(Intents::GUILD_MESSAGES),
			"missing GUILD_MESSAGES"
		);
		assert!(
			intents.contains(Intents::MESSAGE_CONTENT),
			"missing MESSAGE_CONTENT"
		);
	}
}
