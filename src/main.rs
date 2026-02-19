//! Discord bot entry point.
//!
//! All transport details live in `gateway` (WebSocket) and `http` (REST).
//! This file is purely bot logic: reacting to typed events.

mod events;
mod gateway;
mod http;
mod types;

use std::collections::HashSet;
use std::time::Instant;

use tracing::{error, info, warn};

use crate::events::GatewayEvent;
use crate::gateway::GatewayConfig;
use crate::http::DiscordHttpClient;
use crate::types::*;

// ---------------------------------------------------------------------------
// Known guild IDs for fast slash-command registration during development
// ---------------------------------------------------------------------------

const DEV_GUILD_IDS: &[&str] = &[
    "807465587633553409",  // chantey's server
    "1229266524427260057", // beetmash
];

// ---------------------------------------------------------------------------
// Slash command definitions
// ---------------------------------------------------------------------------

fn slash_commands() -> Vec<ApplicationCommand> {
    vec![
        ApplicationCommand {
            id: None,
            name: "ping".to_string(),
            description: "Check bot latency".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "uptime".to_string(),
            description: "See how long the bot has been running".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "roll".to_string(),
            description: "Roll a dice".to_string(),
            options: vec![ApplicationCommandOption {
                name: "sides".to_string(),
                description: "Number of sides (default: 6)".to_string(),
                kind: 4, // INTEGER
                required: false,
                choices: Vec::new(),
            }],
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "serverinfo".to_string(),
            description: "Show server information".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "whoami".to_string(),
            description: "Show info about yourself".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "count".to_string(),
            description: "Count messages in this channel".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "first".to_string(),
            description: "Show the first message ever sent in this channel".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "help".to_string(),
            description: "Show available commands".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "report".to_string(),
            description: "Submit a report via a pop-up form".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "send-logo".to_string(),
            description: "Send the bot logo".to_string(),
            options: Vec::new(),
            kind: 1,
        },
        ApplicationCommand {
            id: None,
            name: "demo-select".to_string(),
            description: "Demo the select menu component".to_string(),
            options: Vec::new(),
            kind: 1,
        },
    ]
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Initialise tracing (respects RUST_LOG env, defaults to info).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    dotenv::dotenv().ok();

    let token = match std::env::var("DISCORD_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            error!("DISCORD_TOKEN environment variable not set");
            std::process::exit(1);
        }
    };

    let start_time = Instant::now();
    let http = DiscordHttpClient::new(&token);

    // Gateway intents:
    // GUILDS(1) | GUILD_MEMBERS(2) | GUILD_PRESENCES(256) |
    // GUILD_MESSAGES(512) | MESSAGE_CONTENT(32768)
    let intents: u32 = 1 | 2 | 256 | 512 | 32768;

    let config = GatewayConfig {
        token: token.clone(),
        intents,
        shard: None, // single-shard for now
    };

    let mut gw = match gateway::connect(config).await {
        Ok(handle) => handle,
        Err(e) => {
            error!(error = %e, "failed to start gateway");
            std::process::exit(1);
        }
    };

    // Bot state - use lazy_static or similar to persist across reconnects in a real app
    // For now, we track if we've registered commands THIS session to avoid duplicates
    let mut bot_user_id: Option<String> = None;
    let mut application_id: Option<String> = None;
    let mut greeted_users: HashSet<String> = HashSet::new();
    let mut greet_channel_id: Option<String> = None;
    // Track commands registered per application_id to avoid duplicates across reconnects
    let mut commands_registered_for_app: Option<String> = None;

    // Main event loop — fully typed, no raw serde_json in sight.
    while let Some(event) = gw.events.recv().await {
        match event {
            // ----- READY -----
            GatewayEvent::Ready(ready) => {
                info!(user = %ready.user.tag(), "bot is ready!");
                bot_user_id = Some(ready.user.id.as_str().to_string());
                application_id = Some(ready.application.id.as_str().to_string());
                info!(guilds = ready.guilds.len(), "connected to guilds");

                // Register slash commands based on SLASH_COMMAND_MODE config.
                // Only register if we haven't already registered for this application_id
                // to avoid duplicates across reconnects.
                if commands_registered_for_app.as_ref() != application_id.as_ref() {
                    if let Some(ref app_id) = application_id {
                        let cmds = slash_commands();
                        let mode = std::env::var("SLASH_COMMAND_MODE")
                            .unwrap_or_else(|_| "guild".to_string());

                        if mode == "global" {
                            // Register globally (takes up to an hour to propagate).
                            match http.bulk_overwrite_global_commands(app_id, &cmds).await {
                                Ok(registered) => {
                                    info!(count = registered.len(), "registered global slash commands (may take up to 1 hour to propagate)");
                                }
                                Err(e) => {
                                    warn!(error = %e, "failed to register global commands");
                                }
                            }
                            // Clear guild commands to avoid duplicates when switching modes
                            for guild_id in DEV_GUILD_IDS {
                                match http
                                    .bulk_overwrite_guild_commands(app_id, guild_id, &[])
                                    .await
                                {
                                    Ok(_) => info!(guild_id, "cleared guild commands"),
                                    Err(e) => {
                                        warn!(guild_id, error = %e, "failed to clear guild commands")
                                    }
                                }
                            }
                        } else {
                            // Default: Register for dev guilds only (fast propagation).
                            info!(mode = %mode, "registering guild-scoped slash commands for fast development");
                            for guild_id in DEV_GUILD_IDS {
                                match http
                                    .bulk_overwrite_guild_commands(app_id, guild_id, &cmds)
                                    .await
                                {
                                    Ok(registered) => {
                                        info!(
                                            guild_id,
                                            count = registered.len(),
                                            "registered guild slash commands"
                                        );
                                    }
                                    Err(e) => {
                                        warn!(guild_id, error = %e, "failed to register guild commands");
                                    }
                                }
                            }
                            // Clear global commands to avoid duplicates when switching modes
                            match http.bulk_overwrite_global_commands(app_id, &[]).await {
                                Ok(_) => info!("cleared global commands"),
                                Err(e) => warn!(error = %e, "failed to clear global commands"),
                            }
                        }

                        commands_registered_for_app = Some(app_id.clone());
                    }
                }
            }

            // ----- GUILD_CREATE -----
            GatewayEvent::GuildCreate(guild) => {
                if greet_channel_id.is_none() {
                    // Pick the first text channel as the greeting channel.
                    if let Some(ch) = guild
                        .channels
                        .iter()
                        .find(|c| c.kind == ChannelType::GuildText)
                    {
                        greet_channel_id = Some(ch.id.as_str().to_string());
                        info!(
                            channel = ch.name.as_deref().unwrap_or("?"),
                            channel_id = %ch.id,
                            "greeting channel set"
                        );
                    }
                }
            }

            // ----- PRESENCE_UPDATE -----
            GatewayEvent::PresenceUpdate(presence) => {
                let status = presence.status.as_deref().unwrap_or("offline");
                if status == "online" {
                    let user_id = presence.user.id.as_str();
                    let is_self = bot_user_id.as_deref() == Some(user_id);

                    if !is_self && !user_id.is_empty() && !greeted_users.contains(user_id) {
                        greeted_users.insert(user_id.to_string());

                        if let Some(ref ch_id) = greet_channel_id {
                            let greeting = format!(
                                "Welcome online, <@{}>! 🎉 Hope you're having a great day!",
                                user_id
                            );
                            if let Err(e) = http.send_message(ch_id, &greeting).await {
                                warn!(error = %e, "failed to send greeting");
                            }
                        }
                    }
                }
            }

            // ----- MESSAGE_CREATE -----
            GatewayEvent::MessageCreate(msg) => {
                if msg.author.bot {
                    continue;
                }

                // Update greet channel if not set.
                if greet_channel_id.is_none() {
                    greet_channel_id = Some(msg.channel_id.as_str().to_string());
                }

                let channel_id = msg.channel_id.as_str();
                let content = msg.content.trim();

                // Check for @BotMention — treat as a command.
                let effective_content = if let Some(ref bid) = bot_user_id {
                    let mention_tag = format!("<@{}>", bid);
                    let mention_tag_nick = format!("<@!{}>", bid);
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
                    } else if msg.mentions_user(bid) {
                        // Mentioned somewhere in the message but not at the start — still
                        // treat as a command if the rest starts with "!".
                        String::new()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Determine the command to handle: either a !command or @mention command.
                let command_text = if content.starts_with('!') {
                    content.to_string()
                } else if !effective_content.is_empty() {
                    // Normalise: if the mention-stripped text doesn't start with !,
                    // prepend it so the match block works uniformly.
                    if effective_content.starts_with('!') {
                        effective_content.clone()
                    } else {
                        format!("!{}", effective_content)
                    }
                } else {
                    String::new()
                };

                if command_text.is_empty() {
                    continue;
                }

                let parts: Vec<&str> = command_text.splitn(2, ' ').collect();
                let command = parts[0];
                let args = parts.get(1).copied().unwrap_or("");

                // All command responses use message_reference to thread the reply.
                let reply =
                    |text: String| CreateMessage::new().content(text).reply_to(msg.id.as_str());

                match command {
                    "!hello" => {
                        let body = reply("Hello, World! 👋".to_string());
                        if let Err(e) = http.create_message(channel_id, &body).await {
                            error!(error = %e, "failed to send !hello reply");
                        }
                    }

                    "!ping" => {
                        let now = chrono::Utc::now();
                        let latency = msg
                            .snowflake_timestamp_ms()
                            .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
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
                                format!("📊 This channel has approximately **{}** messages.", count)
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
                                let ts = if let Ok(dt) =
                                    chrono::DateTime::parse_from_rfc3339(&first_msg.timestamp)
                                {
                                    dt.format("%B %d, %Y at %H:%M UTC").to_string()
                                } else {
                                    first_msg.timestamp.clone()
                                };
                                format!(
                                    "📜 **First message in this channel:**\n> {}\n— *{}* on {}",
                                    first_msg.content, first_msg.author.username, ts
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
                        let text = if let Some(ref guild_id) = msg.guild_id {
                            match http.get_guild(guild_id.as_str()).await {
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
                        let text = format_whoami(&msg.author);
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

            // ----- INTERACTION_CREATE -----
            GatewayEvent::InteractionCreate(interaction) => {
                if let Err(e) =
                    handle_interaction(&http, &interaction, &start_time, &application_id).await
                {
                    error!(error = %e, "failed to handle interaction");
                }
            }

            // ----- Heartbeat ACK (informational) -----
            GatewayEvent::HeartbeatAck => {
                // Already logged at debug level inside the gateway module.
            }

            // ----- Unknown / unhandled events -----
            GatewayEvent::Unknown {
                event_name: Some(ref name),
                ..
            } => {
                // Log unknown dispatch events at trace level to reduce noise.
                tracing::trace!(event = %name, "unhandled gateway event");
            }

            _ => {}
        }
    }

    info!("event stream ended, bot shutting down");
}

// ---------------------------------------------------------------------------
// Interaction handler
// ---------------------------------------------------------------------------

async fn handle_interaction(
    http: &DiscordHttpClient,
    interaction: &Interaction,
    start_time: &Instant,
    application_id: &Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match interaction.kind {
        InteractionType::ApplicationCommand => {
            handle_slash_command(http, interaction, start_time).await
        }
        InteractionType::MessageComponent => {
            handle_component(http, interaction, application_id).await
        }
        InteractionType::ModalSubmit => handle_modal_submit(http, interaction).await,
        InteractionType::Ping => {
            // Respond with PONG.
            let resp = InteractionResponse {
                kind: InteractionCallbackType::Pong,
                data: None,
            };
            http.create_interaction_response(interaction.id.as_str(), &interaction.token, &resp)
                .await?;
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn handle_slash_command(
    http: &DiscordHttpClient,
    interaction: &Interaction,
    start_time: &Instant,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = interaction
        .data
        .as_ref()
        .ok_or("missing interaction data")?;
    let name = data.name.as_deref().unwrap_or("");

    let response = match name {
        "ping" => {
            let text = "🏓 Pong!".to_string();
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    ..Default::default()
                }),
            }
        }

        "uptime" => {
            let elapsed = start_time.elapsed();
            let secs = elapsed.as_secs();
            let text = format!(
                "⏱️ Bot uptime: {}h {}m {}s",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            );
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    ..Default::default()
                }),
            }
        }

        "roll" => {
            let sides = data
                .options
                .iter()
                .find(|o| o.name == "sides")
                .and_then(|o| o.value.as_ref())
                .and_then(|v| v.as_u64())
                .unwrap_or(6) as u32;
            let sides = sides.max(2).min(1000);
            let result = (rand::random::<u32>() % sides) + 1;
            let text = format!("🎲 Rolling a d{}... **{}**!", sides, result);

            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    components: Some(vec![action_row(vec![button(
                        1,
                        "🎲 Reroll",
                        format!("reroll:{}", sides),
                    )])]),
                    ..Default::default()
                }),
            }
        }

        "serverinfo" => {
            let text = if let Some(ref guild_id) = interaction.guild_id {
                match http.get_guild(guild_id.as_str()).await {
                    Ok(guild) => format_guild_info(&guild),
                    Err(e) => format!("❌ Error: {}", e),
                }
            } else {
                "❌ This command only works in a server.".to_string()
            };
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    ..Default::default()
                }),
            }
        }

        "whoami" => {
            let text = match interaction.author() {
                Some(user) => format_whoami(user),
                None => "❌ Couldn't determine your user info.".to_string(),
            };
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    ..Default::default()
                }),
            }
        }

        "count" => {
            let text = if let Some(ref ch_id) = interaction.channel_id {
                match http.count_messages(ch_id.as_str()).await {
                    Ok(count) => {
                        format!("📊 This channel has approximately **{}** messages.", count)
                    }
                    Err(e) => format!("❌ Error: {}", e),
                }
            } else {
                "❌ No channel context.".to_string()
            };
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    ..Default::default()
                }),
            }
        }

        "first" => {
            let text = if let Some(ref ch_id) = interaction.channel_id {
                match http.get_first_message(ch_id.as_str()).await {
                    Ok(first_msg) => {
                        let ts = if let Ok(dt) =
                            chrono::DateTime::parse_from_rfc3339(&first_msg.timestamp)
                        {
                            dt.format("%B %d, %Y at %H:%M UTC").to_string()
                        } else {
                            first_msg.timestamp.clone()
                        };
                        format!(
                            "📜 **First message in this channel:**\n> {}\n— *{}* on {}",
                            first_msg.content, first_msg.author.username, ts
                        )
                    }
                    Err(e) => format!("❌ Error: {}", e),
                }
            } else {
                "❌ No channel context.".to_string()
            };
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(text),
                    ..Default::default()
                }),
            }
        }

        "help" => InteractionResponse {
            kind: InteractionCallbackType::ChannelMessageWithSource,
            data: Some(InteractionCallbackData {
                content: Some(help_text()),
                ..Default::default()
            }),
        },

        "report" => {
            // Show a modal (pop-up form) for the report.
            InteractionResponse {
                kind: InteractionCallbackType::Modal,
                data: Some(InteractionCallbackData {
                    title: Some("📝 Submit a Report".to_string()),
                    custom_id: Some("report_modal".to_string()),
                    components: Some(vec![
                        action_row(vec![text_input(
                            "report_subject",
                            "Subject",
                            1, // Short
                            true,
                        )]),
                        action_row(vec![text_input(
                            "report_body",
                            "Description",
                            2, // Paragraph
                            true,
                        )]),
                    ]),
                    ..Default::default()
                }),
            }
        }

        "send-logo" => {
            // For file uploads, we need to use a follow-up webhook instead of immediate response
            // First, acknowledge the interaction
            let ack_response = InteractionResponse {
                kind: InteractionCallbackType::DeferredChannelMessageWithSource,
                data: None,
            };
            http.create_interaction_response(
                interaction.id.as_str(),
                &interaction.token,
                &ack_response,
            )
            .await?;

            // Send the file using webhook
            if let Some(ref ch_id) = interaction.channel_id {
                let logo_path = "./logo-square.png";
                match tokio::fs::read(logo_path).await {
                    Ok(file_content) => {
                        match http
                            .send_message_with_file(
                                ch_id.as_str(),
                                Some("Here's our logo! 🎨"),
                                "logo-square.png",
                                file_content,
                            )
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                warn!(error = %e, "failed to send logo file");
                                // Try to send error message
                                let _ = http
                                    .send_message(
                                        ch_id.as_str(),
                                        &format!("❌ Failed to send logo: {}", e),
                                    )
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to read logo file");
                        let _ = http
                            .send_message(
                                ch_id.as_str(),
                                &format!("❌ Failed to read logo file: {}", e),
                            )
                            .await;
                    }
                }
            }

            // Return empty since we already handled the response
            return Ok(());
        }

        "demo-select" => {
            use crate::types::{string_select, SelectOption};

            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some("Please select your favorite programming language:".to_string()),
                    components: Some(vec![action_row(vec![string_select(
                        "language_select",
                        "Choose a language...",
                        vec![
                            SelectOption {
                                label: "Rust".to_string(),
                                value: "rust".to_string(),
                                description: Some("Fast, safe, and concurrent".to_string()),
                                emoji: None,
                                default: false,
                            },
                            SelectOption {
                                label: "Python".to_string(),
                                value: "python".to_string(),
                                description: Some("Simple and versatile".to_string()),
                                emoji: None,
                                default: false,
                            },
                            SelectOption {
                                label: "TypeScript".to_string(),
                                value: "typescript".to_string(),
                                description: Some("Typed JavaScript".to_string()),
                                emoji: None,
                                default: false,
                            },
                            SelectOption {
                                label: "Go".to_string(),
                                value: "go".to_string(),
                                description: Some("Simple and efficient".to_string()),
                                emoji: None,
                                default: false,
                            },
                        ],
                    )])]),
                    ..Default::default()
                }),
            }
        }

        _ => {
            info!(command = name, "unknown slash command");
            InteractionResponse {
                kind: InteractionCallbackType::ChannelMessageWithSource,
                data: Some(InteractionCallbackData {
                    content: Some(format!("Unknown command: `/{}`", name)),
                    ..Default::default()
                }),
            }
        }
    };

    http.create_interaction_response(interaction.id.as_str(), &interaction.token, &response)
        .await?;
    Ok(())
}

async fn handle_component(
    http: &DiscordHttpClient,
    interaction: &Interaction,
    _application_id: &Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = interaction
        .data
        .as_ref()
        .ok_or("missing interaction data")?;
    let custom_id = data.custom_id.as_deref().unwrap_or("");

    if custom_id.starts_with("reroll:") {
        let sides: u32 = custom_id
            .strip_prefix("reroll:")
            .and_then(|s| s.parse().ok())
            .unwrap_or(6)
            .max(2)
            .min(1000);

        let result = (rand::random::<u32>() % sides) + 1;
        let text = format!("🎲 Rolling a d{}... **{}**!", sides, result);

        let response = InteractionResponse {
            kind: InteractionCallbackType::UpdateMessage,
            data: Some(InteractionCallbackData {
                content: Some(text),
                components: Some(vec![action_row(vec![button(
                    1,
                    "🎲 Reroll",
                    format!("reroll:{}", sides),
                )])]),
                ..Default::default()
            }),
        };

        http.create_interaction_response(interaction.id.as_str(), &interaction.token, &response)
            .await?;
    } else if !data.values.is_empty() {
        // Select menu response.
        let selected = data.values.join(", ");
        let text = format!("You selected: **{}**", selected);
        let response = InteractionResponse {
            kind: InteractionCallbackType::ChannelMessageWithSource,
            data: Some(InteractionCallbackData {
                content: Some(text),
                flags: Some(64), // EPHEMERAL
                ..Default::default()
            }),
        };
        http.create_interaction_response(interaction.id.as_str(), &interaction.token, &response)
            .await?;
    } else {
        info!(custom_id, "unhandled component interaction");
    }

    Ok(())
}

async fn handle_modal_submit(
    http: &DiscordHttpClient,
    interaction: &Interaction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = interaction
        .data
        .as_ref()
        .ok_or("missing interaction data")?;
    let custom_id = data.custom_id.as_deref().unwrap_or("");

    if custom_id == "report_modal" {
        // Extract values from the submitted modal components.
        let mut subject = String::new();
        let mut body = String::new();

        for row in &data.components {
            for component in &row.components {
                match component.custom_id.as_deref() {
                    Some("report_subject") => {
                        subject = component.value.clone().unwrap_or_default();
                    }
                    Some("report_body") => {
                        body = component.value.clone().unwrap_or_default();
                    }
                    _ => {}
                }
            }
        }

        let author_name = interaction
            .author()
            .map(|u| u.tag())
            .unwrap_or_else(|| "Unknown".to_string());

        let embed = Embed::new()
            .title(format!("📝 Report: {}", subject))
            .description(&body)
            .color(0xFF6600)
            .footer(format!("Submitted by {}", author_name))
            .timestamp(chrono::Utc::now().to_rfc3339());

        let response = InteractionResponse {
            kind: InteractionCallbackType::ChannelMessageWithSource,
            data: Some(InteractionCallbackData {
                content: Some("✅ Report submitted! Thank you.".to_string()),
                embeds: Some(vec![embed]),
                ..Default::default()
            }),
        };

        http.create_interaction_response(interaction.id.as_str(), &interaction.token, &response)
            .await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn format_guild_info(guild: &Guild) -> String {
    let member_count = guild
        .approximate_member_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let online_count = guild
        .approximate_presence_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let owner_id = guild
        .owner_id
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("unknown");
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
        guild.name, member_count, online_count, owner_id, created_at
    )
}

fn format_whoami(user: &User) -> String {
    let avatar_url = user
        .avatar_url()
        .unwrap_or_else(|| "No avatar set".to_string());
    format!(
        "👤 **About You:**\n\
         • **Username:** {}\n\
         • **User ID:** {}\n\
         • **Avatar:** {}",
        user.tag(),
        user.id,
        avatar_url
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
