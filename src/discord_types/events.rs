use beet::prelude::*;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::Message;
use twilight_model::gateway::payload::incoming::GuildCreate;
use twilight_model::gateway::payload::incoming::PresenceUpdate;
use twilight_model::gateway::payload::incoming::Ready;

#[derive(Debug, Clone, EntityEvent)]
pub struct DiscordReady {
	entity: Entity,
	ready: Ready,
}

impl DiscordReady {
	pub fn create(ready: Ready) -> impl FnOnce(Entity) -> Self {
		move |entity| Self { entity, ready }
	}
}

impl std::ops::Deref for DiscordReady {
	type Target = Ready;
	fn deref(&self) -> &Self::Target { &self.ready }
}

// ---------------------------------------------------------------------------
// GUILD_CREATE
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, EntityEvent)]
pub struct DiscordGuildCreate {
	entity: Entity,
	pub guild_create: GuildCreate,
}

impl DiscordGuildCreate {
	pub fn create(guild_create: GuildCreate) -> impl FnOnce(Entity) -> Self {
		move |entity| Self {
			entity,
			guild_create,
		}
	}
}

// ---------------------------------------------------------------------------
// PRESENCE_UPDATE
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, EntityEvent)]
pub struct DiscordPresenceUpdate {
	entity: Entity,
	pub presence: PresenceUpdate,
}

impl DiscordPresenceUpdate {
	pub fn create(presence: PresenceUpdate) -> impl FnOnce(Entity) -> Self {
		move |entity| Self { entity, presence }
	}
}

// ---------------------------------------------------------------------------
// MESSAGE_CREATE
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, EntityEvent)]
pub struct DiscordMessage {
	entity: Entity,
	pub message: Message,
}

impl DiscordMessage {
	pub fn create(message: Message) -> impl FnOnce(Entity) -> Self {
		move |entity| Self { entity, message }
	}
}

impl std::ops::Deref for DiscordMessage {
	type Target = Message;
	fn deref(&self) -> &Self::Target { &self.message }
}

#[derive(Debug, Clone, EntityEvent)]
pub struct DiscordInteraction {
	entity: Entity,
	pub interaction: Interaction,
}

impl DiscordInteraction {
	pub fn create(interaction: Interaction) -> impl FnOnce(Entity) -> Self {
		move |entity| Self {
			entity,
			interaction,
		}
	}
}

impl std::ops::Deref for DiscordInteraction {
	type Target = Interaction;
	fn deref(&self) -> &Self::Target { &self.interaction }
}
