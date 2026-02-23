//! Event handlers for the Discord bot.
//!
//! Each public function in this module handles one category of gateway event.
//! Handlers receive an [`AsyncWorld`] for reading/writing Bevy [`Resource`]s
//! and a [`DiscordHttpClient`] for calling the Discord REST API.
//!
//! This module also contains slash-command definitions and small formatting
//! helpers that were previously inlined in `lib.rs`.

use beet::prelude::AsyncWorld;
use tracing::{error, info, warn};

use crate::bot::{BotState, GreetState};
use crate::http::DiscordHttpClient;
use crate::types::*;

// ---------------------------------------------------------------------------
// Slash command definitions
// ---------------------------------------------------------------------------

/// Returns the list of slash commands to register with Discord.
pub fn slash_commands() -> Vec<ApplicationCommand> {
    use crate::types::application::command::CommandOptionType;

    vec![
        ApplicationCommandBuilder::chat_input("ping", "Check bot latency").build(),
        ApplicationCommandBuilder::chat_input("uptime", "See how long the bot has been running")
            .build(),
        ApplicationCommandBuilder::chat_input("roll", "Roll a dice")
            .simple_option(
                CommandOptionType::Integer,
                "sides",
                "Number of sides (default: 6)",
                false,
            )
            .build(),
        ApplicationCommandBuilder::chat_input("serverinfo", "Show server information").build(),
        ApplicationCommandBuilder::chat_input("whoami", "Show info about yourself").build(),
        ApplicationCommandBuilder::chat_input("count", "Count messages in this channel").build(),
        ApplicationCommandBuilder::chat_input(
            "first",
            "Show the first message ever sent in this channel",
        )
        .build(),
        ApplicationCommandBuilder::chat_input("help", "Show available commands").build(),
        ApplicationCommandBuilder::chat_input("report", "Submit a report via a pop-up form")
            .build(),
        ApplicationCommandBuilder::chat_input("send-logo", "Send the bot logo").build(),
        ApplicationCommandBuilder::chat_input("demo-select", "Demo the select menu component")
            .build(),
    ]
}

// ---------------------------------------------------------------------------
// READY handler
// ---------------------------------------------------------------------------

/// Called when the bot receives the READY event from the gateway.
///
/// Stores identity information in [`BotState`] and registers slash commands
/// globally (once per session).
pub async fn on_ready(world: &AsyncWorld, http: &DiscordHttpClient, ready: ReadyEvent) {
    info!(user = %ready.user.tag(), guilds = ready.guilds.len(), "bot is ready!");

    let bot_user_id = ready.user.id;
    let app_id = ready.application.id;

    // Store identity in BotState, and check whether commands are already registered.
    let already_registered = world
        .with_resource_then::<BotState, _>(move |mut state| {
            state.bot_user_id = Some(bot_user_id);
            state.application_id = Some(app_id);
            state.commands_registered
        })
        .await;

    if !already_registered {
        let cmds = slash_commands();
        match http.bulk_overwrite_global_commands(app_id, &cmds).await {
            Ok(registered) => {
                info!(count = registered.len(), "registered global slash commands");
                world.with_resource::<BotState>(|mut state| {
                    state.commands_registered = true;
                });
            }
            Err(e) => {
                warn!(error = %e, "failed to register global commands");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GUILD_CREATE handler
// ---------------------------------------------------------------------------

/// Called when we receive a full guild object after READY.
///
/// Picks the first text channel as the greeting channel if one hasn't been
/// set yet.
pub async fn on_guild_create(world: &AsyncWorld, guild: Guild) {
    let has_greet_channel = world
        .with_resource_then::<GreetState, _>(|state| state.greet_channel_id.is_some())
        .await;

    if !has_greet_channel {
        if let Some(ch) = guild
            .channels
            .iter()
            .find(|c| c.kind == ChannelType::GuildText)
        {
            let channel_id = ch.id;
            let channel_name = ch.name.clone().unwrap_or_else(|| "?".to_string());
            world.with_resource::<GreetState>(move |mut state| {
                info!(
                    channel = %channel_name,
                    channel_id = %channel_id,
                    "greeting channel set"
                );
                state.greet_channel_id = Some(channel_id);
            });
        }
    }
}

// ---------------------------------------------------------------------------
// PRESENCE_UPDATE handler
// ---------------------------------------------------------------------------

/// Called when a user's presence changes.
///
/// Sends a one-time greeting when a user comes online for the first time
/// this session.
pub async fn on_presence_update(
    world: &AsyncWorld,
    http: &DiscordHttpClient,
    presence: PresenceUpdate,
) {
    let status = presence.status.as_deref().unwrap_or("offline");
    if status != "online" {
        return;
    }

    let user_id = presence.user.id;

    // Check whether this is the bot itself and whether we've already greeted.
    let is_self = world
        .with_resource_then::<BotState, _>(move |state| state.bot_user_id == Some(user_id))
        .await;

    if is_self {
        return;
    }

    // Now check GreetState (separate resource access to keep borrows clean).
    let (already_greeted, greet_channel) = world
        .with_resource_then::<GreetState, _>(move |state| {
            let already = state.greeted_users.contains(&user_id);
            (already, state.greet_channel_id)
        })
        .await;

    if already_greeted {
        return;
    }

    // Mark as greeted.
    world.with_resource::<GreetState>(move |mut state| {
        state.greeted_users.insert(user_id);
    });

    if let Some(ch_id) = greet_channel {
        let greeting = format!(
            "Welcome online, <@{}>! 🎉 Hope you're having a great day!",
            user_id
        );
        if let Err(e) = http.send_message(ch_id, &greeting).await {
            warn!(error = %e, "failed to send greeting");
        }
    }
}

// ---------------------------------------------------------------------------
// MESSAGE_CREATE handler
// ---------------------------------------------------------------------------

/// Called when a non-bot user sends a message.
///
/// Handles `!` prefix commands and @-mention commands.
pub async fn on_message(world: &AsyncWorld, http: &DiscordHttpClient, msg: Message) {
    info!(
        message_id = %msg.id,
        author = %msg.author.tag(),
        channel_id = %msg.channel_id,
        content = %msg.content,
        "handling message"
    );

    // Update greet channel if not yet set.
    let channel_id = msg.channel_id;
    world.with_resource::<GreetState>(move |mut state| {
        if state.greet_channel_id.is_none() {
            state.greet_channel_id = Some(channel_id);
        }
    });

    let content = msg.content.trim();

    // Read bot_user_id + start_time from BotState.
    let (bot_user_id, start_time) = world
        .with_resource_then::<BotState, _>(|state| (state.bot_user_id, state.start_time))
        .await;

    // Determine effective command text from @mention or ! prefix.
    let effective_content = if let Some(bid) = bot_user_id {
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
            String::new()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let command_text = if content.starts_with('!') {
        content.to_string()
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
        return;
    }

    let parts: Vec<&str> = command_text.splitn(2, ' ').collect();
    let command = parts[0];
    let args = parts.get(1).copied().unwrap_or("");

    let reply = |text: String| CreateMessage::new().content(text).reply_to(msg.id);

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
                    let ts_str = first_msg.timestamp.as_str();
                    let ts = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                        dt.format("%B %d, %Y at %H:%M UTC").to_string()
                    } else {
                        ts_str.to_string()
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
            let text = if let Some(guild_id) = msg.guild_id {
                match http.get_guild(guild_id).await {
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

// ---------------------------------------------------------------------------
// INTERACTION_CREATE handler
// ---------------------------------------------------------------------------

/// Top-level interaction dispatcher.
pub async fn on_interaction(
    world: &AsyncWorld,
    http: &DiscordHttpClient,
    interaction: &Interaction,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match interaction.kind {
        InteractionType::ApplicationCommand => handle_slash_command(world, http, interaction).await,
        InteractionType::MessageComponent => handle_component(http, interaction).await,
        InteractionType::ModalSubmit => handle_modal_submit(http, interaction).await,
        InteractionType::Ping => {
            let resp = InteractionResponse {
                kind: InteractionCallbackType::Pong,
                data: None,
            };
            http.create_interaction_response(interaction.id, &interaction.token, &resp)
                .await?;
            Ok(())
        }
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Slash command handler
// ---------------------------------------------------------------------------

/// Extract command data from an interaction.
///
/// Twilight models `InteractionData` as an enum; slash commands carry the
/// `ApplicationCommand` variant. This helper pulls out the name and options.
fn command_info(interaction: &Interaction) -> Option<(&str, &[CommandDataOption])> {
    match interaction.data.as_ref()? {
        InteractionData::ApplicationCommand(data) => Some((&data.name, &data.options)),
        _ => None,
    }
}

/// Extract a u64 option value from a list of command data options.
fn get_option_u64(options: &[CommandDataOption], name: &str) -> Option<u64> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandOptionValue::Integer(v) => Some(*v as u64),
            CommandOptionValue::Number(v) => Some(*v as u64),
            _ => None,
        })
}

async fn handle_slash_command(
    world: &AsyncWorld,
    http: &DiscordHttpClient,
    interaction: &Interaction,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (name, options) = command_info(interaction).ok_or("missing interaction data")?;

    let start_time = world
        .with_resource_then::<BotState, _>(|state| state.start_time)
        .await;

    let response = match name {
        "ping" => text_response("🏓 Pong!"),

        "uptime" => {
            let elapsed = start_time.elapsed();
            let secs = elapsed.as_secs();
            text_response(format!(
                "⏱️ Bot uptime: {}h {}m {}s",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            ))
        }

        "roll" => {
            let sides = get_option_u64(options, "sides").unwrap_or(6) as u32;
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
            let text = if let Some(guild_id) = interaction.guild_id {
                match http.get_guild(guild_id).await {
                    Ok(guild) => format_guild_info(&guild),
                    Err(e) => format!("❌ Error: {}", e),
                }
            } else {
                "❌ This command only works in a server.".to_string()
            };
            text_response(text)
        }

        "whoami" => {
            let text = match interaction.author() {
                Some(user) => format_whoami(user),
                None => "❌ Couldn't determine your user info.".to_string(),
            };
            text_response(text)
        }

        "count" => {
            #[allow(deprecated)]
            let text = if let Some(ch_id) = interaction.channel_id {
                match http.count_messages(ch_id).await {
                    Ok(count) => {
                        format!("📊 This channel has approximately **{}** messages.", count)
                    }
                    Err(e) => format!("❌ Error: {}", e),
                }
            } else {
                "❌ No channel context.".to_string()
            };
            text_response(text)
        }

        "first" => {
            #[allow(deprecated)]
            let text = if let Some(ch_id) = interaction.channel_id {
                match http.get_first_message(ch_id).await {
                    Ok(first_msg) => {
                        let ts_str = first_msg.timestamp.as_str();
                        let ts = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                            dt.format("%B %d, %Y at %H:%M UTC").to_string()
                        } else {
                            ts_str.to_string()
                        };
                        format!(
                            "📜 **First message in this channel:**\n> {}\n— *{}* on {}",
                            first_msg.content, first_msg.author.name, ts
                        )
                    }
                    Err(e) => format!("❌ Error: {}", e),
                }
            } else {
                "❌ No channel context.".to_string()
            };
            text_response(text)
        }

        "help" => text_response(help_text()),

        "report" => InteractionResponse {
            kind: InteractionCallbackType::Modal,
            data: Some(InteractionCallbackData {
                title: Some("📝 Submit a Report".to_string()),
                custom_id: Some("report_modal".to_string()),
                components: Some(vec![
                    action_row(vec![text_input("report_subject", "Subject", 1, true)]),
                    action_row(vec![text_input("report_body", "Description", 2, true)]),
                ]),
                ..Default::default()
            }),
        },

        "send-logo" => {
            // Acknowledge first, then send file as a follow-up.
            let ack = InteractionResponse {
                kind: InteractionCallbackType::DeferredChannelMessageWithSource,
                data: None,
            };
            http.create_interaction_response(interaction.id, &interaction.token, &ack)
                .await?;

            #[allow(deprecated)]
            if let Some(ch_id) = interaction.channel_id {
                match std::fs::read("./logo-square.png") {
                    Ok(file_content) => {
                        if let Err(e) = http
                            .send_message_with_file(
                                ch_id,
                                Some("Here's our logo! 🎨"),
                                "logo-square.png",
                                file_content,
                            )
                            .await
                        {
                            warn!(error = %e, "failed to send logo file");
                            let _ = http
                                .send_message(ch_id, &format!("❌ Failed to send logo: {}", e))
                                .await;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to read logo file");
                        let _ = http
                            .send_message(ch_id, &format!("❌ Failed to read logo file: {}", e))
                            .await;
                    }
                }
            }
            // Already responded via deferred + follow-up.
            return Ok(());
        }

        "demo-select" => InteractionResponse {
            kind: InteractionCallbackType::ChannelMessageWithSource,
            data: Some(InteractionCallbackData {
                content: Some("Please select your favorite programming language:".to_string()),
                components: Some(vec![action_row(vec![string_select(
                    "language_select",
                    "Choose a language...",
                    vec![
                        SelectMenuOption {
                            default: false,
                            description: Some("Fast, safe, and concurrent".to_string()),
                            emoji: None,
                            label: "Rust".to_string(),
                            value: "rust".to_string(),
                        },
                        SelectMenuOption {
                            default: false,
                            description: Some("Simple and versatile".to_string()),
                            emoji: None,
                            label: "Python".to_string(),
                            value: "python".to_string(),
                        },
                        SelectMenuOption {
                            default: false,
                            description: Some("Typed JavaScript".to_string()),
                            emoji: None,
                            label: "TypeScript".to_string(),
                            value: "typescript".to_string(),
                        },
                        SelectMenuOption {
                            default: false,
                            description: Some("Simple and efficient".to_string()),
                            emoji: None,
                            label: "Go".to_string(),
                            value: "go".to_string(),
                        },
                    ],
                )])]),
                ..Default::default()
            }),
        },

        _ => {
            info!(command = name, "unknown slash command");
            text_response(format!("Unknown command: `/{}`", name))
        }
    };

    http.create_interaction_response(interaction.id, &interaction.token, &response)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Component interaction handler
// ---------------------------------------------------------------------------

/// Extract component data from an interaction.
fn component_info(interaction: &Interaction) -> Option<(&str, &[String])> {
    match interaction.data.as_ref()? {
        InteractionData::MessageComponent(data) => Some((&data.custom_id, &data.values)),
        _ => None,
    }
}

async fn handle_component(
    http: &DiscordHttpClient,
    interaction: &Interaction,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (custom_id, values) = component_info(interaction).ok_or("missing interaction data")?;

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

        http.create_interaction_response(interaction.id, &interaction.token, &response)
            .await?;
    } else if !values.is_empty() {
        let selected = values.join(", ");
        let text = format!("You selected: **{}**", selected);
        let response = InteractionResponse {
            kind: InteractionCallbackType::ChannelMessageWithSource,
            data: Some(InteractionCallbackData {
                content: Some(text),
                flags: Some(64), // EPHEMERAL
                ..Default::default()
            }),
        };
        http.create_interaction_response(interaction.id, &interaction.token, &response)
            .await?;
    } else {
        info!(custom_id, "unhandled component interaction");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Modal submit handler
// ---------------------------------------------------------------------------

/// Extract text input values from a modal submit interaction.
fn modal_text_inputs(interaction: &Interaction) -> Option<(String, Vec<(String, String)>)> {
    use crate::types::application::interaction::modal::ModalInteractionComponent;

    match interaction.data.as_ref()? {
        InteractionData::ModalSubmit(data) => {
            let custom_id = data.custom_id.clone();
            let mut inputs = Vec::new();
            for row in &data.components {
                // Each top-level component in a modal is an ActionRow
                if let ModalInteractionComponent::ActionRow(action_row) = row {
                    for component in &action_row.components {
                        if let ModalInteractionComponent::TextInput(ti) = component {
                            inputs.push((ti.custom_id.clone(), ti.value.clone()));
                        }
                    }
                }
            }
            Some((custom_id, inputs))
        }
        _ => None,
    }
}

async fn handle_modal_submit(
    http: &DiscordHttpClient,
    interaction: &Interaction,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (custom_id, text_inputs) =
        modal_text_inputs(interaction).ok_or("missing interaction data")?;

    if custom_id == "report_modal" {
        let mut subject = String::new();
        let mut body = String::new();

        for (id, value) in &text_inputs {
            match id.as_str() {
                "report_subject" => subject = value.clone(),
                "report_body" => body = value.clone(),
                _ => {}
            }
        }

        let author_name = interaction
            .author()
            .map(|u| u.tag())
            .unwrap_or_else(|| "Unknown".to_string());

        let embed = EmbedBuilder::new()
            .title(format!("📝 Report: {}", subject))
            .description(&body)
            .color(0xFF6600)
            .footer(format!("Submitted by {}", author_name))
            .timestamp(chrono::Utc::now().to_rfc3339())
            .build();

        let response = InteractionResponse {
            kind: InteractionCallbackType::ChannelMessageWithSource,
            data: Some(InteractionCallbackData {
                content: Some("✅ Report submitted! Thank you.".to_string()),
                embeds: Some(vec![embed]),
                ..Default::default()
            }),
        };

        http.create_interaction_response(interaction.id, &interaction.token, &response)
            .await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

/// Shorthand for a simple text interaction response.
fn text_response(text: impl Into<String>) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionCallbackType::ChannelMessageWithSource,
        data: Some(InteractionCallbackData {
            content: Some(text.into()),
            ..Default::default()
        }),
    }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- slash_commands() --------------------------------------------------

    #[test]
    fn slash_commands_returns_expected_count() {
        let cmds = slash_commands();
        assert_eq!(cmds.len(), 11);
    }

    #[test]
    fn slash_commands_names_are_unique() {
        let cmds = slash_commands();
        let mut names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), cmds.len(), "duplicate command names found");
    }

    #[test]
    fn slash_commands_all_have_descriptions() {
        for cmd in slash_commands() {
            assert!(
                !cmd.description.is_empty(),
                "command '{}' has empty description",
                cmd.name
            );
        }
    }

    #[test]
    fn roll_command_has_sides_option() {
        let cmds = slash_commands();
        let roll = cmds.iter().find(|c| c.name == "roll").expect("no /roll");
        assert_eq!(roll.options.len(), 1);
        assert_eq!(roll.options[0].name, "sides");
        assert!(matches!(
            roll.options[0].kind,
            crate::types::application::command::CommandOptionType::Integer
        ));
        assert_eq!(roll.options[0].required, Some(false));
    }

    // -- text_response() ---------------------------------------------------

    #[test]
    fn text_response_sets_content() {
        let resp = text_response("hello");
        assert_eq!(
            resp.data.as_ref().unwrap().content.as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn text_response_uses_channel_message_type() {
        let resp = text_response("x");
        assert!(matches!(
            resp.kind,
            InteractionCallbackType::ChannelMessageWithSource
        ));
    }

    #[test]
    fn text_response_has_no_extras() {
        let resp = text_response("x");
        let data = resp.data.unwrap();
        assert!(data.embeds.is_none());
        assert!(data.components.is_none());
        assert!(data.flags.is_none());
    }

    // -- format_guild_info() -----------------------------------------------

    #[test]
    fn format_guild_info_includes_guild_name() {
        let guild: Guild = serde_json::from_value(serde_json::json!({
            "id": "123",
            "name": "Test Server",
            "icon": null,
            "owner_id": "456",
            "approximate_member_count": 42,
            "approximate_presence_count": 10,
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
        .expect("valid guild JSON");
        let text = format_guild_info(&guild);
        assert!(text.contains("Test Server"), "missing guild name");
        assert!(text.contains("42"), "missing member count");
        assert!(text.contains("10"), "missing online count");
        assert!(text.contains("<@456>"), "missing owner mention");
    }

    #[test]
    fn format_guild_info_handles_missing_counts() {
        let guild: Guild = serde_json::from_value(serde_json::json!({
            "id": "1",
            "name": "Empty",
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
        .expect("valid guild JSON");
        let text = format_guild_info(&guild);
        assert!(
            text.contains("unknown"),
            "missing 'unknown' for absent counts"
        );
    }

    // -- format_whoami() ---------------------------------------------------

    #[test]
    fn format_whoami_includes_username_and_id() {
        let user: User = serde_json::from_value(serde_json::json!({
            "id": "789",
            "username": "alice",
            "discriminator": "0001",
            "avatar": null,
            "bot": false,
            "global_name": null,
        }))
        .expect("valid user JSON");
        let text = format_whoami(&user);
        assert!(text.contains("alice"), "missing username");
        assert!(text.contains("789"), "missing user id");
    }

    // -- help_text() -------------------------------------------------------

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
    fn help_text_mentions_all_slash_commands() {
        let text = help_text();
        for name in slash_commands().iter().map(|c| c.name.as_str()) {
            assert!(
                text.contains(&format!("/{}", name)),
                "help text missing /{}",
                name
            );
        }
    }
}
