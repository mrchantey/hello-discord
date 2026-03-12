//! Discord helpers — re-exports from [`twilight_model`] plus our custom types
//! and extension traits.
//!
//! This module replaces the old `discord_types` fork of twilight-model.
//! Instead of maintaining a full copy of twilight's type definitions, we now
//! depend on the published `twilight-model` crate and layer our custom
//! additions on top.

/// Custom types that don't exist in `twilight-model` (outbound message body,
/// rate-limit info, etc.)
mod custom;
pub use custom::*;
pub mod events;
mod ext;
pub use ext::*;

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
pub use twilight_model::id::marker::ApplicationMarker;
pub use twilight_model::id::marker::AttachmentMarker;
pub use twilight_model::id::marker::ChannelMarker;
pub use twilight_model::id::marker::CommandMarker;
pub use twilight_model::id::marker::GuildMarker;
pub use twilight_model::id::marker::InteractionMarker;
pub use twilight_model::id::marker::MessageMarker;
pub use twilight_model::id::marker::RoleMarker;
pub use twilight_model::id::marker::UserMarker;
pub use twilight_model::id::Id;

// ---- User -----------------------------------------------------------------
pub use twilight_model::user::CurrentUser;
pub use twilight_model::user::User;

// ---- Channel / Message ----------------------------------------------------
pub use twilight_model::channel::message::component::ActionRow;
pub use twilight_model::channel::message::component::Button;
pub use twilight_model::channel::message::component::ButtonStyle;
pub use twilight_model::channel::message::component::Component;
pub use twilight_model::channel::message::component::ComponentType;
pub use twilight_model::channel::message::component::SelectMenu;
pub use twilight_model::channel::message::component::SelectMenuOption;
pub use twilight_model::channel::message::component::SelectMenuType;
pub use twilight_model::channel::message::component::TextInput;
pub use twilight_model::channel::message::component::TextInputStyle;
pub use twilight_model::channel::message::embed::Embed;
pub use twilight_model::channel::message::embed::EmbedAuthor;
pub use twilight_model::channel::message::embed::EmbedField;
pub use twilight_model::channel::message::embed::EmbedFooter;
pub use twilight_model::channel::message::embed::EmbedImage;
pub use twilight_model::channel::message::embed::EmbedThumbnail;
pub use twilight_model::channel::message::Mention;
pub use twilight_model::channel::message::Message;
pub use twilight_model::channel::message::MessageFlags;
pub use twilight_model::channel::message::MessageReference;
pub use twilight_model::channel::Attachment;
pub use twilight_model::channel::Channel;
pub use twilight_model::channel::ChannelType;

// ---- Guild ----------------------------------------------------------------
pub use twilight_model::guild::Guild;
pub use twilight_model::guild::Member;
pub use twilight_model::guild::PartialMember;
pub use twilight_model::guild::UnavailableGuild;

// ---- Presence / Gateway ---------------------------------------------------
pub use twilight_model::gateway::payload::incoming::Ready;
pub use twilight_model::gateway::presence::Activity;
pub use twilight_model::gateway::presence::Status;
pub use twilight_model::gateway::OpCode;

// ---- Interactions ---------------------------------------------------------
pub use twilight_model::application::interaction::application_command::CommandData;
pub use twilight_model::application::interaction::application_command::CommandDataOption;
pub use twilight_model::application::interaction::application_command::CommandOptionValue;
pub use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
pub use twilight_model::application::interaction::modal::ModalInteractionData;
pub use twilight_model::application::interaction::Interaction;
pub use twilight_model::application::interaction::InteractionData;
pub use twilight_model::application::interaction::InteractionType;

// ---- Interaction responses (from twilight — no custom duplicates) ----------
pub use twilight_model::http::interaction::InteractionResponse;
pub use twilight_model::http::interaction::InteractionResponseData;
pub use twilight_model::http::interaction::InteractionResponseType;

// ---- Application commands (registration) ----------------------------------
pub use twilight_model::application::command::Command;
pub use twilight_model::application::command::Command as ApplicationCommand;
pub use twilight_model::application::command::CommandOption as ApplicationCommandOption;
pub use twilight_model::application::command::CommandOptionChoice as ApplicationCommandOptionChoice;
pub use twilight_model::application::command::CommandOptionType;
pub use twilight_model::application::command::CommandType as ApplicationCommandType;
