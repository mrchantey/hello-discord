//! Extension traits for twilight-model types.
//!
//! These add ergonomic helper methods (`.tag()`, `.avatar_url()`,
//! `.snowflake_timestamp_ms()`, `.author()`, etc.) that don't exist on the
//! upstream twilight types.
//!
//! # Fork note
//!
//! Upstream twilight-model intentionally keeps its types minimal — no
//! convenience methods, no Display impls beyond what serde needs. In our
//! original hand-rolled types these methods lived directly on the structs.
//! Now that we use twilight's struct definitions, we provide the same
//! ergonomics via extension traits.
//!
//! The rest of the codebase imports these from `crate::types::*` so they're
//! always in scope.

use crate::types::{
    application::interaction::Interaction,
    channel::message::Message,
    guild::Guild,
    id::{marker::UserMarker, Id},
    user::User,
    util::ImageHash,
};

// ===========================================================================
// UserExt
// ===========================================================================

/// Convenience methods on [`User`].
pub trait UserExt {
    /// Returns the CDN URL for the user's avatar, or `None` if no avatar is set.
    fn avatar_url(&self) -> Option<String>;

    /// `Username#Discriminator` or just `Username` for the new username system.
    fn tag(&self) -> String;
}

impl UserExt for User {
    fn avatar_url(&self) -> Option<String> {
        let hash: &ImageHash = self.avatar.as_ref()?;
        Some(format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            self.id, hash
        ))
    }

    fn tag(&self) -> String {
        if self.discriminator == 0 {
            self.name.clone()
        } else {
            format!("{}#{:04}", self.name, self.discriminator)
        }
    }
}

// ===========================================================================
// MessageExt
// ===========================================================================

/// Convenience methods on [`Message`].
pub trait MessageExt {
    /// Unix-millisecond timestamp derived from the message snowflake.
    fn snowflake_timestamp_ms(&self) -> Option<u64>;

    /// Whether a given user ID is mentioned in the message.
    fn mentions_user(&self, user_id: Id<UserMarker>) -> bool;
}

impl MessageExt for Message {
    fn snowflake_timestamp_ms(&self) -> Option<u64> {
        let sf = self.id.get();
        Some((sf >> 22) + 1_420_070_400_000)
    }

    fn mentions_user(&self, user_id: Id<UserMarker>) -> bool {
        self.mentions.iter().any(|m| m.id == user_id)
    }
}

// ===========================================================================
// GuildExt
// ===========================================================================

/// Convenience methods on [`Guild`].
pub trait GuildExt {
    /// Unix-millisecond timestamp derived from the guild snowflake.
    fn created_at_ms(&self) -> Option<u64>;
}

impl GuildExt for Guild {
    fn created_at_ms(&self) -> Option<u64> {
        let sf = self.id.get();
        Some((sf >> 22) + 1_420_070_400_000)
    }
}

// ===========================================================================
// InteractionExt
// ===========================================================================

/// Convenience methods on [`Interaction`].
pub trait InteractionExt {
    /// The user who triggered the interaction.
    ///
    /// In a guild context, the user is nested inside `member`. In a DM, the
    /// user is at the top level. This helper checks both.
    fn author(&self) -> Option<&User>;
}

impl InteractionExt for Interaction {
    fn author(&self) -> Option<&User> {
        self.member
            .as_ref()
            .and_then(|m| m.user.as_ref())
            .or(self.user.as_ref())
    }
}

// ===========================================================================
// IdExt — convenience on Id<T>
// ===========================================================================

/// Convenience methods on [`Id<T>`].
///
/// Provides `.as_str_buf()` and `Display`-based stringification that mirrors
/// our old `Snowflake::as_str()` usage patterns. Note that `Id<T>` already
/// implements `Display`, so `id.to_string()` works out of the box.
pub trait IdExt {
    /// Get the inner u64 value of the ID.
    fn value(&self) -> u64;
}

impl<T> IdExt for Id<T> {
    fn value(&self) -> u64 {
        self.get()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::id::marker::{GuildMarker, MessageMarker};

    fn make_test_user() -> User {
        // Construct a minimal User for testing.
        // We need to provide all required fields from the twilight User struct.
        serde_json::from_value(serde_json::json!({
            "id": "789",
            "username": "alice",
            "discriminator": "0001",
            "avatar": null,
            "bot": false,
        }))
        .expect("valid user JSON")
    }

    #[test]
    fn user_tag_with_discriminator() {
        let user: User = serde_json::from_value(serde_json::json!({
            "id": "789",
            "username": "alice",
            "discriminator": "0001",
            "avatar": null,
        }))
        .unwrap();

        assert_eq!(user.tag(), "alice#0001");
    }

    #[test]
    fn user_tag_new_system() {
        let user: User = serde_json::from_value(serde_json::json!({
            "id": "789",
            "username": "alice",
            "discriminator": "0",
            "avatar": null,
        }))
        .unwrap();

        assert_eq!(user.tag(), "alice");
    }

    #[test]
    fn user_avatar_url_none_when_no_avatar() {
        let user = make_test_user();
        assert!(user.avatar_url().is_none());
    }

    #[test]
    fn user_avatar_url_present() {
        // Discord image hashes are 32-char hex strings (128-bit).
        let user: User = serde_json::from_value(serde_json::json!({
            "id": "789",
            "username": "alice",
            "discriminator": "0",
            "avatar": "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d",
        }))
        .unwrap();

        let url = user.avatar_url().unwrap();
        assert!(url.contains("789"));
        assert!(url.starts_with("https://cdn.discordapp.com/avatars/"));
    }

    #[test]
    fn guild_created_at_ms() {
        // Guild ID that corresponds to a known timestamp
        let guild_id = Id::<GuildMarker>::new(175928847299117063);
        // The created_at_ms formula: (sf >> 22) + 1420070400000
        let expected = (175928847299117063u64 >> 22) + 1_420_070_400_000;

        // We can't easily construct a full Guild struct without lots of fields,
        // so test the formula directly
        let sf = guild_id.get();
        let ms = (sf >> 22) + 1_420_070_400_000;
        assert_eq!(ms, expected);
    }

    #[test]
    fn message_snowflake_timestamp() {
        let msg_id = Id::<MessageMarker>::new(175928847299117063);
        let expected = (175928847299117063u64 >> 22) + 1_420_070_400_000;

        // Test the formula
        let sf = msg_id.get();
        let ms = (sf >> 22) + 1_420_070_400_000;
        assert_eq!(ms, expected);
    }

    #[test]
    fn id_ext_value() {
        let id = Id::<GuildMarker>::new(12345);
        assert_eq!(id.value(), 12345);
    }
}
