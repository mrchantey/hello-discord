//! Types that don't exist in twilight-model.
//!
//! These are specific to our implementation and cover outbound message bodies
//! and rate-limit tracking. Gateway event types (including presence updates)
//! are now provided directly by `twilight-model`.

use serde::Deserialize;
use serde::Serialize;

use twilight_model::channel::message::component::Component;
use twilight_model::channel::message::embed::Embed;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::marker::MessageMarker;
use twilight_model::id::Id;

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
	pub fn new() -> Self { Self::default() }

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
}
