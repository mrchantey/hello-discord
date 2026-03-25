//! Core bot infrastructure: Bevy Resources, gateway bridge, and async event loop.
//!
//! This module owns the "engine" of the bot — connecting to Discord's gateway,
//! polling events, and dispatching them to handler functions. All mutable state
//! lives in Bevy [`Resource`]s accessed through [`AsyncWorld`], so no manual
//! mutexes are needed in the bot layer.

use crate::prelude::*;
use beet::prelude::*;
use twilight_model::gateway::event::DispatchEvent;
use twilight_model::gateway::event::GatewayEvent;
use twilight_model::gateway::Intents;


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
			GatewayEvent::Dispatch(_, dispatch) => match dispatch {
				DispatchEvent::Ready(ready) => {
					entity.trigger(DiscordReady::create(ready));
				}
				DispatchEvent::GuildCreate(guild_create) => {
					entity.trigger(DiscordGuildCreate::create(*guild_create));
				}
				DispatchEvent::PresenceUpdate(presence) => {
					entity.trigger(DiscordPresenceUpdate::create(*presence));
				}
				DispatchEvent::MessageCreate(msg) => {
					entity.trigger(DiscordMessage::create(msg.0));
				}
				DispatchEvent::InteractionCreate(interaction) => {
					entity.trigger(DiscordInteraction::create(interaction.0));
				}
				DispatchEvent::Resumed => {
					// known event, no-op
				}
				other => {
					tracing::warn!(event = ?other, "unhandled dispatch event");
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
