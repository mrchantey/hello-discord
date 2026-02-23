//! Types that don't exist in twilight-model.
//!
//! These are specific to `hello-discord` and cover gateway envelopes,
//! outbound message bodies, rate-limit tracking, and READY event payloads
//! that twilight handles differently (via its own gateway crate).

use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;

use crate::types::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
    Id,
};
use crate::types::user::User;

// ---------------------------------------------------------------------------
// Gateway payload (the raw WebSocket envelope) — not in twilight-model
// ---------------------------------------------------------------------------

/// Raw gateway payload envelope.
///
/// Every message on the Discord WebSocket is wrapped in this structure.
/// Twilight handles this in its `twilight-gateway` crate, but we do our own
/// gateway handling so we need the raw envelope.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GatewayPayload {
    pub op: u8,
    pub d: Option<serde_json::Value>,
    pub s: Option<u64>,
    pub t: Option<String>,
}

// ---------------------------------------------------------------------------
// READY event payload
// ---------------------------------------------------------------------------

/// The READY event data sent by the gateway after a successful IDENTIFY.
///
/// Twilight parses this inside its gateway crate. We define it here because
/// we run our own gateway loop.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReadyEvent {
    pub v: u8,
    pub user: User,
    pub session_id: String,
    pub resume_gateway_url: String,
    #[serde(default)]
    pub guilds: Vec<crate::types::guild::UnavailableGuild>,
    pub application: ReadyApplication,
}

/// Minimal application object embedded in [`ReadyEvent`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReadyApplication {
    pub id: Id<crate::types::id::marker::ApplicationMarker>,
    pub flags: Option<u64>,
}

// ---------------------------------------------------------------------------
// PRESENCE_UPDATE event payload
// ---------------------------------------------------------------------------

/// A simplified PRESENCE_UPDATE payload.
///
/// Twilight's full `Presence` type is in `gateway::presence` and requires
/// many more fields. Our bot only cares about the user, guild, status, and
/// activities, so we keep a streamlined version here.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PresenceUpdate {
    pub user: PartialUser,
    pub guild_id: Option<Id<GuildMarker>>,
    pub status: Option<String>,
    #[serde(default)]
    pub activities: Vec<crate::types::gateway::presence::Activity>,
}

/// Partial user object received in events like PRESENCE_UPDATE.
///
/// Only the `id` is guaranteed; other fields may be absent.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartialUser {
    pub id: Id<UserMarker>,
    pub username: Option<String>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

// ---------------------------------------------------------------------------
// Outbound message body (for REST POST /channels/{id}/messages)
// ---------------------------------------------------------------------------

/// Body for creating a new message via the REST API.
///
/// Uses a builder pattern for ergonomic construction:
///
/// ```ignore
/// let msg = CreateMessage::new()
///     .content("Hello!")
///     .reply_to("123456789");
/// ```ignore
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<crate::types::channel::message::embed::Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<CreateMessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<crate::types::channel::message::component::Component>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

impl CreateMessage {
    /// Create a new empty message body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the text content of the message.
    pub fn content(mut self, text: impl Into<String>) -> Self {
        self.content = Some(text.into());
        self
    }

    /// Append an embed to the message.
    #[allow(dead_code)]
    pub fn embed(mut self, embed: crate::types::channel::message::embed::Embed) -> Self {
        self.embeds.get_or_insert_with(Vec::new).push(embed);
        self
    }

    /// Mark the message as a reply to another message.
    pub fn reply_to(mut self, message_id: Id<MessageMarker>) -> Self {
        self.message_reference = Some(CreateMessageReference {
            message_id: Some(message_id),
            channel_id: None,
            guild_id: None,
            fail_if_not_exists: false,
        });
        self
    }

    /// Append a component row to the message.
    pub fn component_row(
        mut self,
        row: crate::types::channel::message::component::Component,
    ) -> Self {
        self.components.get_or_insert_with(Vec::new).push(row);
        self
    }
}

/// Simplified message reference for outbound messages.
///
/// Uses typed `Id<T>` markers for compile-time safety.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateMessageReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Id<MessageMarker>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Id<ChannelMarker>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Id<GuildMarker>>,
    #[serde(default)]
    pub fail_if_not_exists: bool,
}

// ---------------------------------------------------------------------------
// Interaction response types
// ---------------------------------------------------------------------------

/// An interaction response sent back to Discord.
///
/// This mirrors twilight's `InteractionResponse` but uses our simplified
/// `InteractionCallbackData` that's easier to construct with `Default`.
#[derive(Debug, Clone, Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub kind: InteractionCallbackType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<InteractionCallbackData>,
}

/// The type of callback for an interaction response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr)]
#[repr(u8)]
pub enum InteractionCallbackType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    DeferredChannelMessageWithSource = 5,
    DeferredUpdateMessage = 6,
    UpdateMessage = 7,
    ApplicationCommandAutocompleteResult = 8,
    Modal = 9,
}

// Allow deserializing as well (useful in tests / echo scenarios).
impl<'de> Deserialize<'de> for InteractionCallbackType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = u8::deserialize(deserializer)?;
        match v {
            1 => Ok(Self::Pong),
            4 => Ok(Self::ChannelMessageWithSource),
            5 => Ok(Self::DeferredChannelMessageWithSource),
            6 => Ok(Self::DeferredUpdateMessage),
            7 => Ok(Self::UpdateMessage),
            8 => Ok(Self::ApplicationCommandAutocompleteResult),
            9 => Ok(Self::Modal),
            _ => Err(serde::de::Error::custom(format!(
                "unknown InteractionCallbackType: {}",
                v
            ))),
        }
    }
}

/// Data payload for an interaction callback.
///
/// This is our simplified version that supports `Default` for easy
/// construction with struct update syntax (`..Default::default()`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionCallbackData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<crate::types::channel::message::embed::Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<crate::types::channel::message::component::Component>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// For modal responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Rate-limit info parsed from response headers
// ---------------------------------------------------------------------------

/// Rate-limit metadata extracted from Discord REST API response headers.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub remaining: Option<u32>,
    #[allow(dead_code)]
    pub reset_at: Option<f64>,
    pub reset_after: Option<f64>,
    pub bucket: Option<String>,
    pub is_global: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_message_builder() {
        let msg = CreateMessage::new()
            .content("hello")
            .reply_to(Id::new(12345));

        assert_eq!(msg.content.as_deref(), Some("hello"));
        assert!(msg.message_reference.is_some());
        let reference = msg.message_reference.unwrap();
        assert_eq!(reference.message_id.map(|id| id.get()), Some(12345));
    }

    #[test]
    fn create_message_serializes() {
        let msg = CreateMessage::new().content("test");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"content\":\"test\""));
        // Optional fields should be absent
        assert!(!json.contains("embeds"));
        assert!(!json.contains("components"));
        assert!(!json.contains("flags"));
    }

    #[test]
    fn gateway_payload_deserializes() {
        let json = r#"{"op":0,"d":null,"s":1,"t":"READY"}"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, 0);
        assert_eq!(payload.s, Some(1));
        assert_eq!(payload.t.as_deref(), Some("READY"));
    }

    #[test]
    fn interaction_callback_type_roundtrip() {
        let ty = InteractionCallbackType::ChannelMessageWithSource;
        let json = serde_json::to_string(&ty).unwrap();
        assert_eq!(json, "4");
        let parsed: InteractionCallbackType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ty);
    }
}
