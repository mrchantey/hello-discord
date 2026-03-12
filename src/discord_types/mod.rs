//! Discord helpers — re-exports from [`twilight_model`] plus our custom types
//! and extension traits.
//!
//! This module replaces the old `discord_types` fork of twilight-model.
//! Instead of maintaining a full copy of twilight's type definitions, we now
//! depend on the published `twilight-model` crate and layer our custom
//! additions on top.

/// Custom types that don't exist in `twilight-model` (outbound message body,
/// rate-limit info, simplified presence, etc.)
mod custom;
pub use custom::*;

/// Typed gateway events and dispatch parsing.
mod events;
pub use events::*;

/// Extension traits and helpers for twilight-model types.
pub mod ext;

// ===========================================================================
// Re-exports from twilight-model
// ===========================================================================

// ---- Top-level modules (so code can do `discord_types::application::...`) ----
pub use twilight_model::application;
pub use twilight_model::channel;
pub use twilight_model::gateway;
pub use twilight_model::guild;
pub use twilight_model::http;
pub use twilight_model::id;
pub use twilight_model::oauth;
pub use twilight_model::poll;
pub use twilight_model::user;
pub use twilight_model::util;
pub use twilight_model::voice;

// ===========================================================================
// Convenience re-exports (flat imports for `use crate::discord_types::*`)
// ===========================================================================

// ---- IDs ------------------------------------------------------------------
pub use twilight_model::id::marker::{
    ApplicationMarker, AttachmentMarker, ChannelMarker, CommandMarker, GuildMarker,
    InteractionMarker, MessageMarker, RoleMarker, UserMarker,
};
pub use twilight_model::id::Id;

// ---- User -----------------------------------------------------------------
pub use twilight_model::user::{CurrentUser, User};

// ---- Channel / Message ----------------------------------------------------
pub use twilight_model::channel::message::MessageFlags;
pub use twilight_model::channel::{
    message::{
        component::{
            ActionRow, Button, ButtonStyle, Component, ComponentType, SelectMenu, SelectMenuOption,
            SelectMenuType, TextInput, TextInputStyle,
        },
        embed::{Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedImage, EmbedThumbnail},
        Mention, Message, MessageReference,
    },
    Attachment, Channel, ChannelType,
};

// ---- Guild ----------------------------------------------------------------
pub use twilight_model::guild::{Guild, Member, PartialMember, UnavailableGuild};

// ---- Presence / Gateway ---------------------------------------------------
pub use twilight_model::gateway::payload::incoming::Ready;
pub use twilight_model::gateway::presence::{Activity, Status};
pub use twilight_model::gateway::OpCode;

// ---- Interactions ---------------------------------------------------------
pub use twilight_model::application::interaction::{
    application_command::{CommandData, CommandDataOption, CommandOptionValue},
    message_component::MessageComponentInteractionData,
    modal::ModalInteractionData,
    Interaction, InteractionData, InteractionType,
};

// ---- Interaction responses (from twilight — no custom duplicates) ----------
pub use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

// ---- Application commands (registration) ----------------------------------
pub use twilight_model::application::command::CommandOptionType;
pub use twilight_model::application::command::{
    Command, Command as ApplicationCommand, CommandOption as ApplicationCommandOption,
    CommandOptionChoice as ApplicationCommandOptionChoice, CommandType as ApplicationCommandType,
};

// ---- Extension traits & helpers (our additions) ---------------------------
pub use self::ext::*;
