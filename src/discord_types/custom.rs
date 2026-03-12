//! Types that don't exist in twilight-model.
//!
//! These are specific to our implementation and cover outbound message bodies,
//! rate-limit tracking, and simplified presence payloads that twilight handles
//! differently (via its own gateway crate).

use serde::{Deserialize, Serialize};

use twilight_model::channel::message::component::Component;
use twilight_model::channel::message::embed::Embed;
use twilight_model::gateway::presence::Activity;
use twilight_model::gateway::OpCode;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker};
use twilight_model::id::Id;

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
    pub activities: Vec<Activity>,
}

/// An event we received from the gateway that we don't have a typed variant for.
///
/// Uses the strongly-typed [`OpCode`] instead of a raw `u8`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnknownEvent {
    pub event_name: Option<String>,
    pub op: OpCode,
    pub data: Option<serde_json::Value>,
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
///     .reply_to(Id::new(123456789));
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<CreateMessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<Component>>,
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
    pub fn embed(mut self, embed: Embed) -> Self {
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
    pub fn component_row(mut self, row: Component) -> Self {
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
    fn unknown_event_uses_opcode() {
        let event = UnknownEvent {
            event_name: Some("SOME_EVENT".into()),
            op: OpCode::Dispatch,
            data: None,
        };
        assert_eq!(event.op, OpCode::Dispatch);
    }
}
