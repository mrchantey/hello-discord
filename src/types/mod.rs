//! Discord API types — a focused fork of [`twilight-model`] v0.17.
//!
//! # Fork notes
//!
//! This module is a **non-git fork** of the
//! [twilight-model](https://github.com/twilight-rs/twilight/tree/main/twilight-model)
//! crate (v0.17.1, commit snapshot taken June 2025). The source was copied into
//! `src/types/` and adapted to live inside the `hello-discord` crate rather
//! than as a standalone dependency.
//!
//! ## Why fork instead of depend?
//!
//! `hello-discord` is a *framework* crate. Every dependency we add is imposed
//! on every consumer. `twilight-model` transitively pulls in `serde-value`,
//! `time`, and `ordered-float`, and its design decisions (15-field command
//! structs, deprecated fields on someone else's schedule) ripple through our
//! API surface. By forking we get the type-safety wins (typed IDs, typed
//! component/interaction enums) while keeping full control over ergonomics and
//! dependency footprint.
//!
//! ## What changed from upstream twilight-model
//!
//! | Area | Change |
//! |---|---|
//! | **Crate paths** | All `crate::` references rewritten to `crate::types::` |
//! | **`time` dependency** | Behind the `timestamps` feature flag. Without it, `Timestamp` is a thin `String` newtype with serde passthrough. |
//! | **`serde-value` dependency** | Replaced with `serde_json::Value` + `serde_json::from_value` in custom deserializers. |
//! | **Unused modules** | `oauth`, `poll`, `voice`, and large gateway sub-modules are behind the `full-model` feature flag. |
//! | **Builders** | `ApplicationCommand` and `Embed` have ergonomic builder patterns (see [`builders`]). |
//! | **Custom types** | `GatewayPayload`, `CreateMessage`, `RateLimitInfo`, and other types that don't exist in twilight live in [`custom`]. |
//! | **Extension traits** | `UserExt`, `MessageExt`, etc. live in [`ext`]. |
//!
//! ## Syncing with upstream
//!
//! To update from a newer twilight-model release:
//!
//! 1. Download the new source into a temp directory
//! 2. Diff against `twilight-model/` (the reference copy kept at the repo root)
//! 3. Apply relevant changes to `src/types/`, keeping the adaptations above
//! 4. Run `sed -i 's/crate::/crate::types::/g'` on any newly-added files
//! 5. Update timestamp and version in this doc comment
//!
//! The `twilight-model/` directory at the project root is kept as a reference
//! copy to make future diffs easier.

// ---------------------------------------------------------------------------
// Lint configuration (matches upstream twilight style)
// ---------------------------------------------------------------------------
// Note: #![warn(...)] and #![allow(...)] are crate-level attributes and cannot
// appear in a non-root module. We rely on the crate-level settings in lib.rs
// and add per-item allows as needed.

// ===========================================================================
// Sub-modules — forked from twilight-model
// ===========================================================================

/// Application commands and interactions.
pub mod application;

/// Channels, messages, embeds, and components.
pub mod channel;

/// Gateway presence types (and optionally full gateway events).
pub mod gateway;

/// Guilds, members, roles, and permissions.
pub mod guild;

/// HTTP-specific types (interaction responses).
pub mod http;

/// Type-safe IDs with marker types.
pub mod id;

/// Utility types (timestamps, image hashes, hex colors).
pub mod util;

/// Custom serde visitors shared across modules.
pub(crate) mod visitor;

// ---- Modules referenced by core types (always compiled) -------------------
// These are smaller modules that core types (Interaction, Message, etc.)
// depend on. They must always be available even though we don't use them
// directly in our bot logic.

/// OAuth2 application and team types.
///
/// Referenced by `application::interaction` (for `ApplicationIntegrationMap`)
/// and `application::command` (for `ApplicationIntegrationType`).
pub mod oauth;

/// Poll types.
///
/// Referenced by `channel::message::Message` (the `poll` field).
pub mod poll;

/// User types (from twilight).
pub mod user;

/// Voice types.
///
/// Referenced by gateway payload types.
pub mod voice;

// ---- Test module from upstream (only compiled in test builds) ---------------
#[cfg(test)]
mod test;

// ===========================================================================
// Our custom types, builders, and extensions
// ===========================================================================

/// Builder patterns for ergonomic type construction.
pub mod builders;

/// Types that don't exist in twilight-model.
pub mod custom;

/// Extension traits for twilight types.
pub mod ext;

// ===========================================================================
// Convenience re-exports
// ===========================================================================
// The rest of the codebase does `use crate::types::*` so we re-export the
// most commonly used items here.

// ---- IDs ------------------------------------------------------------------
pub use self::id::marker::{
    ApplicationMarker, AttachmentMarker, ChannelMarker, CommandMarker, GuildMarker,
    InteractionMarker, MessageMarker, RoleMarker, UserMarker,
};
pub use self::id::Id;

// ---- User -----------------------------------------------------------------
pub use self::user::User;

// ---- Channel / Message ----------------------------------------------------
pub use self::channel::message::MessageFlags;
pub use self::channel::{
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
pub use self::guild::{Guild, Member, PartialMember, UnavailableGuild};

// ---- Presence / Gateway ---------------------------------------------------
pub use self::gateway::presence::{Activity, Status};

// ---- Interactions ---------------------------------------------------------
pub use self::application::interaction::{
    application_command::{CommandData, CommandDataOption, CommandOptionValue},
    message_component::MessageComponentInteractionData,
    modal::ModalInteractionData,
    Interaction, InteractionData, InteractionType,
};

// ---- Interaction responses (what we send back) ----------------------------
// We use our own simplified InteractionResponse / InteractionCallbackType /
// InteractionCallbackData from the `custom` module rather than twilight's
// `http::interaction` types, because ours support `Default` and are easier
// to construct with struct-update syntax (`..Default::default()`).
pub use self::custom::{InteractionCallbackData, InteractionCallbackType, InteractionResponse};

// ---- Application commands (registration) ----------------------------------
pub use self::application::command::CommandOptionType;
pub use self::application::command::{
    Command as ApplicationCommand, CommandOption as ApplicationCommandOption,
    CommandOptionChoice as ApplicationCommandOptionChoice, CommandType as ApplicationCommandType,
};

// ---- Builders (our additions) ---------------------------------------------
pub use self::builders::{ApplicationCommandBuilder, EmbedBuilder};

// ---- Custom types (our additions) -----------------------------------------
pub use self::custom::{
    CreateMessage, GatewayPayload, PartialUser, PresenceUpdate, RateLimitInfo, ReadyApplication,
    ReadyEvent,
};

// ---- Extension traits (our additions) -------------------------------------
pub use self::ext::{GuildExt, InteractionExt, MessageExt, UserExt};

// ---- Component helpers (our additions) ------------------------------------
pub use self::builders::{action_row, button, link_button, string_select, text_input};
