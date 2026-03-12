use crate::discord_io::bot::start_gateway_listener;
use crate::prelude::Message;
use crate::prelude::*;
use beet::prelude::*;

pub fn default_bot() -> impl Bundle {}

#[derive(Component)]
#[component(on_add=on_add)]
pub struct DiscordBot {
    /// The bot's token, usually loaded from the environment at startup.
    token: String,
    /// The event handlers for each gateway event type.
    pub handlers: DiscordHandlers,
}

#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
    let entity = cx.entity;
    world
        .commands()
        .queue_async(async move |world| start_gateway_listener(world.entity(entity)).await);
}

pub struct DiscordHandlers {
    /// We've successfully identified / resumed — bot is ready.
    pub on_ready: Tool<ReadyEvent, ()>,
    /// Full guild object lazily sent after READY.
    pub on_guild_create: Tool<Guild, ()>,
    /// A message was created in a channel we can see.
    pub on_message_create: Tool<Message, ()>,
    /// A user's presence (online/idle/dnd/offline) changed.
    pub on_presence_update: Tool<PresenceUpdate, ()>,
    /// An interaction was created (slash command, button, select, modal submit).
    pub on_interaction_create: Tool<Interaction, ()>,
    /// Heartbeat ACK from the gateway (op 11).
    pub on_heartbeat_ack: Tool<(), ()>,
    /// The gateway is asking us to heartbeat immediately (op 1).
    pub on_heartbeat_request: Tool<(), ()>,
    /// Gateway told us to reconnect (op 7).
    pub on_reconnect: Tool<(), ()>,
    /// Session has been invalidated (op 9).
    pub on_invalid_session: Tool<bool, ()>,
    /// An event we received but don't have a typed variant for yet.
    pub on_unknown: Tool<UnknownEvent, ()>,
}

fn log_event<T: std::fmt::Debug>(value: FuncToolIn<T>) -> Result {
    let input = value.input;
    println!("Received event: {input:#?}");
    Ok(())
}

impl Default for DiscordHandlers {
    fn default() -> Self {
        Self {
            on_ready: func_tool(log_event),
            on_guild_create: func_tool(log_event),
            on_message_create: func_tool(log_event),
            on_presence_update: func_tool(log_event),
            on_interaction_create: func_tool(log_event),
            on_heartbeat_ack: func_tool(log_event),
            on_heartbeat_request: func_tool(log_event),
            on_reconnect: func_tool(log_event),
            on_invalid_session: func_tool(log_event),
            on_unknown: func_tool(log_event),
        }
    }
}

impl Default for DiscordBot {
    fn default() -> Self {
        Self {
            token: env_ext::var("DISCORD_TOKEN").unwrap(),
            handlers: default(),
        }
    }
}
