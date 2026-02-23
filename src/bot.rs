//! Core bot infrastructure: Bevy Resources, gateway bridge, and async event loop.
//!
//! This module owns the "engine" of the bot — connecting to Discord's gateway,
//! polling events, and dispatching them to handler functions. All mutable state
//! lives in Bevy [`Resource`]s accessed through [`AsyncWorld`], so no manual
//! mutexes are needed in the bot layer.

use beet::prelude::*;
use tracing::{error, info, warn};

use crate::events::GatewayEvent;
use crate::gateway::{self, GatewayConfig};
use crate::handlers;
use crate::http::DiscordHttpClient;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Core bot identity and lifecycle state.
#[derive(Resource)]
pub struct BotState {
    /// The bot's own user ID (set on READY).
    pub bot_user_id: Option<String>,
    /// The application ID (set on READY).
    pub application_id: Option<String>,
    /// Whether slash commands have been registered this session.
    pub commands_registered: bool,
    /// Timestamp of when the bot started.
    pub start_time: Instant,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            bot_user_id: None,
            application_id: None,
            commands_registered: false,
            start_time: Instant::now(),
        }
    }
}

/// State for the "greet users who come online" feature.
#[derive(Resource, Default)]
pub struct GreetState {
    /// Channel to send greeting messages in.
    pub greet_channel_id: Option<String>,
    /// Users we've already greeted this session (to avoid spamming).
    pub greeted_users: HashSet<String>,
}

// ---------------------------------------------------------------------------
// Gateway intents
// ---------------------------------------------------------------------------

/// Build the gateway intents bitmask.
///
/// GUILDS(1) | GUILD_MEMBERS(2) | GUILD_PRESENCES(256) |
/// GUILD_MESSAGES(512) | MESSAGE_CONTENT(32768)
fn gateway_intents() -> u32 {
    1 | 2 | 256 | 512 | 32768
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
pub async fn start(world: AsyncWorld) -> Result {
    dotenv::dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").map_err(|_| {
        error!("DISCORD_TOKEN environment variable not set");
        "DISCORD_TOKEN environment variable not set"
    })?;

    // Create the HTTP client (cheap to clone — Arc internals).
    let http = DiscordHttpClient::new(&token);

    // Insert state into the Bevy world as Resources.
    world.insert_resource_then(BotState::default()).await;
    world.insert_resource_then(GreetState::default()).await;

    // Connect to the Discord gateway.
    let config = GatewayConfig {
        token,
        intents: gateway_intents(),
        shard: None, // single-shard
    };

    let gw = gateway::connect(config).await.map_err(|e| {
        error!(error = %e, "failed to start gateway");
        e
    })?;

    info!("gateway connected, entering event loop");

    // ----- Main event loop -----
    while let Ok(event) = gw.events.recv().await {
        match event {
            GatewayEvent::Ready(ready) => {
                handlers::on_ready(&world, &http, ready).await;
            }

            GatewayEvent::GuildCreate(guild) => {
                handlers::on_guild_create(&world, guild).await;
            }

            GatewayEvent::PresenceUpdate(presence) => {
                handlers::on_presence_update(&world, &http, presence).await;
            }

            GatewayEvent::MessageCreate(msg) => {
                if msg.author.bot {
                    continue;
                }
                handlers::on_message(&world, &http, msg).await;
            }

            GatewayEvent::InteractionCreate(interaction) => {
                if let Err(e) = handlers::on_interaction(&world, &http, &interaction).await {
                    error!(error = %e, "failed to handle interaction");
                }
            }

            // Heartbeat ACK — already logged at debug level in gateway module.
            GatewayEvent::HeartbeatAck => {}

            // Reconnect / InvalidSession are handled internally by the gateway driver.
            GatewayEvent::Reconnect | GatewayEvent::InvalidSession(_) => {}

            GatewayEvent::HeartbeatRequest => {}

            GatewayEvent::Unknown {
                event_name: Some(ref name),
                ..
            } => {
                tracing::trace!(event = %name, "unhandled gateway event");
            }

            _ => {}
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

    // -- BotState ----------------------------------------------------------

    #[test]
    fn bot_state_default_has_no_identity() {
        let state = BotState::default();
        assert!(state.bot_user_id.is_none());
        assert!(state.application_id.is_none());
        assert!(!state.commands_registered);
    }

    #[test]
    fn bot_state_start_time_is_recent() {
        let before = Instant::now();
        let state = BotState::default();
        let after = Instant::now();
        assert!(state.start_time >= before);
        assert!(state.start_time <= after);
    }

    #[test]
    fn bot_state_tracks_identity() {
        let mut state = BotState::default();
        state.bot_user_id = Some("12345".to_string());
        state.application_id = Some("67890".to_string());
        assert_eq!(state.bot_user_id.as_deref(), Some("12345"));
        assert_eq!(state.application_id.as_deref(), Some("67890"));
    }

    #[test]
    fn bot_state_commands_registered_flag() {
        let mut state = BotState::default();
        assert!(!state.commands_registered);
        state.commands_registered = true;
        assert!(state.commands_registered);
    }

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
        assert!(!state.greeted_users.contains("user_a"));
        state.greeted_users.insert("user_a".to_string());
        assert!(state.greeted_users.contains("user_a"));
        assert!(!state.greeted_users.contains("user_b"));
    }

    #[test]
    fn greet_state_no_duplicate_greetings() {
        let mut state = GreetState::default();
        assert!(state.greeted_users.insert("user_a".to_string()));
        // Second insert returns false — user already present.
        assert!(!state.greeted_users.insert("user_a".to_string()));
        assert_eq!(state.greeted_users.len(), 1);
    }

    #[test]
    fn greet_state_channel_id_can_be_set() {
        let mut state = GreetState::default();
        state.greet_channel_id = Some("chan_123".to_string());
        assert_eq!(state.greet_channel_id.as_deref(), Some("chan_123"));
    }

    // -- gateway_intents() -------------------------------------------------

    #[test]
    fn gateway_intents_includes_required_bits() {
        let intents = gateway_intents();
        assert_ne!(intents & 1, 0, "missing GUILDS");
        assert_ne!(intents & 2, 0, "missing GUILD_MEMBERS");
        assert_ne!(intents & 256, 0, "missing GUILD_PRESENCES");
        assert_ne!(intents & 512, 0, "missing GUILD_MESSAGES");
        assert_ne!(intents & 32768, 0, "missing MESSAGE_CONTENT");
    }
}
