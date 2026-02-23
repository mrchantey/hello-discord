//! Markers for various resource types, such as channels or users.
//!
//! Markers themselves perform no logical action, and are only used to
//! ensure that IDs of incorrect types aren't used. If IDs were only 64-bit
//! integers then a role's ID may be erroneously used in the place of where
//! a user's ID is required; by using markers it can be ensured that only an
//! ID with a [`RoleMarker`] can be used where a role's ID is required.

// DEVELOPMENT: When adding a new marker, be sure to add its implementation to
// `util/snowflake`.

/// Marker for application IDs.
///
/// Types such as [`Message::application_id`] or [`Guild::application_id`]
/// use this ID marker.
///
/// [`Guild::application_id`]: crate::types::guild::Guild::application_id
/// [`Message::application_id`]: crate::types::channel::Message::application_id
#[derive(Debug)]
#[non_exhaustive]
pub struct ApplicationMarker;

/// Marker for attachment IDs.
///
/// Types such as [`Attachment`] use this ID marker.
///
/// [`Attachment`]: crate::types::channel::Attachment
#[derive(Debug)]
#[non_exhaustive]
pub struct AttachmentMarker;

/// Marker for audit log entry IDs.
///
/// Types such as [`AuditLogEntry`] use this ID marker.
///
/// [`AuditLogEntry`]: crate::types::guild::audit_log::AuditLogEntry
#[derive(Debug)]
#[non_exhaustive]
pub struct AuditLogEntryMarker;

/// Marker for auto moderation rule IDs.
///
/// Types such as [`AutoModerationRule`] use this ID marker.
///
/// [`AutoModerationRule`]: crate::types::guild::auto_moderation::AutoModerationRule
#[derive(Debug)]
#[non_exhaustive]
pub struct AutoModerationRuleMarker;

/// Marker for channel IDs.
///
/// Types such as [`Channel`] or [`GatewayReaction`] use this ID marker.
///
/// [`Channel`]: crate::types::channel::Channel
/// [`GatewayReaction`]: crate::types::gateway::GatewayReaction
#[derive(Debug)]
#[non_exhaustive]
pub struct ChannelMarker;

/// Marker for command IDs.
///
/// Types such as [`Command`] use this ID marker.
///
/// [`Command`]: crate::types::application::command::Command
#[derive(Debug)]
#[non_exhaustive]
pub struct CommandMarker;

/// Marker for command versions.
///
/// Types such as [`Command`] use this ID marker.
///
/// [`Command`]: crate::types::application::command::Command
#[derive(Debug)]
#[non_exhaustive]
pub struct CommandVersionMarker;

/// Marker for emoji IDs.
///
/// Types such as [`Emoji`] or [`ReactionType`] use this ID marker.
///
/// [`Emoji`]: crate::types::guild::Emoji
/// [`ReactionType`]: crate::types::channel::message::ReactionType
#[derive(Debug)]
#[non_exhaustive]
pub struct EmojiMarker;

/// Marker for entitlement IDs.
///
/// Types such as [`Entitlement`] use this ID marker.
///
/// [`Entitlement`]: crate::types::application::monetization::entitlement::Entitlement
#[derive(Debug)]
#[non_exhaustive]
pub struct EntitlementMarker;

/// Marker for entitlement SKU IDs.
///
/// Types such as [`Sku`] use this ID marker.
///
/// [`Sku`]: crate::types::application::monetization::sku::Sku
#[derive(Debug)]
#[non_exhaustive]
pub struct SkuMarker;

/// Marker for generic IDs.
///
/// Types such as [`AuditLogChange::Id`] or [`CommandOptionValue`] use this
/// ID marker.
///
/// [`AuditLogChange::Id`]: crate::types::guild::audit_log::AuditLogChange::Id
/// [`CommandOptionValue`]: crate::types::application::interaction::application_command::CommandOptionValue
#[derive(Debug)]
#[non_exhaustive]
pub struct GenericMarker;

/// Marker for guild IDs.
///
/// Types such as [`Guild`] or [`Message`] use this ID marker.
///
/// [`Guild`]: crate::types::guild::Guild
/// [`Message`]: crate::types::channel::Message
#[derive(Debug)]
#[non_exhaustive]
pub struct GuildMarker;

/// Marker for integration IDs.
///
/// Types such as [`GuildIntegration`] or [`RoleTags`] use this ID marker.
///
/// [`GuildIntegration`]: crate::types::guild::GuildIntegration
/// [`RoleTags`]: crate::types::guild::RoleTags
#[derive(Debug)]
#[non_exhaustive]
pub struct IntegrationMarker;

/// Marker for interaction IDs.
///
/// Types such as [`Interaction`] or [`MessageInteraction`] use this ID
/// marker.
///
/// [`Interaction`]: crate::types::application::interaction::Interaction
/// [`MessageInteraction`]: crate::types::channel::message::MessageInteraction
#[derive(Debug)]
#[non_exhaustive]
pub struct InteractionMarker;

/// Marker for message IDs.
///
/// Types such as [`Message`] or [`GatewayReaction`] use this ID marker.
///
/// [`Message`]: crate::types::channel::Message
/// [`GatewayReaction`]: crate::types::gateway::GatewayReaction
#[derive(Debug)]
#[non_exhaustive]
pub struct MessageMarker;

/// Marker for OAuth SKU IDs.
///
/// Types such as [`Application`] use this ID marker.
///
/// [`Application`]: crate::types::oauth::Application
#[derive(Debug)]
#[non_exhaustive]
pub struct OauthSkuMarker;

/// Marker for OAuth team IDs.
///
/// Types such as [`Team`] or [`TeamMember`] use this ID marker.
///
/// [`Team`]: crate::types::oauth::team::Team
/// [`TeamMember`]: crate::types::oauth::team::TeamMember
#[derive(Debug)]
#[non_exhaustive]
pub struct OauthTeamMarker;

/// Marker for onboarding prompt IDs.
///
/// Types such as [`OnboardingPrompt`] use this ID marker.
///
/// [`OnboardingPrompt`]: crate::types::guild::onboarding::OnboardingPrompt
#[derive(Debug)]
#[non_exhaustive]
pub struct OnboardingPromptMarker;

/// Marker for onboarding prompt option IDs.
///
/// Types such as [`OnboardingPromptOption`] use this ID marker.
///
/// [`OnboardingPromptOption`]: crate::types::guild::onboarding::OnboardingPromptOption
#[derive(Debug)]
#[non_exhaustive]
pub struct OnboardingPromptOptionMarker;

/// Marker for role IDs.
///
/// Types such as [`Member`] or [`Role`] use this ID marker.
///
/// [`Member`]: crate::types::guild::Member
/// [`Role`]: crate::types::guild::Role
#[derive(Debug)]
#[non_exhaustive]
pub struct RoleMarker;

/// Marker for scheduled event IDs.
///
/// Types such as [`GuildScheduledEvent`] use this ID marker.
///
/// [`GuildScheduledEvent`]: crate::types::guild::scheduled_event::GuildScheduledEvent
#[derive(Debug)]
#[non_exhaustive]
pub struct ScheduledEventMarker;

/// Marker for scheduled event entity IDs.
///
/// Types such as [`GuildScheduledEvent`] use this ID marker.
///
/// [`GuildScheduledEvent`]: crate::types::guild::scheduled_event::GuildScheduledEvent
#[derive(Debug)]
#[non_exhaustive]
pub struct ScheduledEventEntityMarker;

/// Marker for stage IDs.
///
/// Types such as [`StageInstance`] use this ID marker.
///
/// [`StageInstance`]: crate::types::channel::StageInstance
#[derive(Debug)]
#[non_exhaustive]
pub struct StageMarker;

/// Marker for sticker banner asset IDs.
///
/// Types such as [`StickerPack`] use this ID marker.
///
/// [`StickerPack`]: crate::types::channel::message::sticker::StickerPack
#[derive(Debug)]
#[non_exhaustive]
pub struct StickerBannerAssetMarker;

/// Marker for sticker IDs.
///
/// Types such as [`Message`] or [`Sticker`] use this ID marker.
///
/// [`Message`]: crate::types::channel::Message
/// [`Sticker`]: crate::types::channel::message::sticker::Sticker
#[derive(Debug)]
#[non_exhaustive]
pub struct StickerMarker;

/// Marker for sticker pack IDs.
///
/// Types such as [`Sticker`] or [`StickerPack`] use this ID marker.
///
/// [`Sticker`]: crate::types::channel::message::sticker::Sticker
/// [`StickerPack`]: crate::types::channel::message::sticker::StickerPack
#[derive(Debug)]
#[non_exhaustive]
pub struct StickerPackMarker;

/// Marker for sticker pack SKU IDs.
///
/// Types such as [`StickerPack`] use this ID marker.
///
/// [`StickerPack`]: crate::types::channel::message::sticker::StickerPack
#[derive(Debug)]
#[non_exhaustive]
pub struct StickerPackSkuMarker;

/// Marker for SKU IDs.
///
/// Types such as [`RoleTags`] use this ID marker.
///
/// [`RoleTags`]: crate::types::guild::RoleTags
#[derive(Debug)]
#[non_exhaustive]
pub struct RoleSubscriptionSkuMarker;

/// Marker for forum tag IDs.
///
/// Types such as [`ForumTag`] use this ID marker.
///
/// [`ForumTag`]: crate::types::channel::forum::ForumTag
#[derive(Debug)]
#[non_exhaustive]
pub struct TagMarker;

/// Marker for user IDs.
///
/// Types such as [`Channel`] or [`User`] use this ID marker.
///
/// [`Channel`]: crate::types::channel::Channel
/// [`User`]: crate::types::user::User
#[derive(Debug)]
#[non_exhaustive]
pub struct UserMarker;

/// Marker for webhook IDs.
///
/// Types such as [`Webhook`] use this ID marker.
///
/// [`Webhook`]: crate::types::channel::webhook::Webhook
#[derive(Debug)]
#[non_exhaustive]
pub struct WebhookMarker;

/// SKU ID marker for avatar decoration data.
///
/// Types such as [`AvatarDecorationData`] use this ID marker.
///
/// [`AvatarDecorationData`]: crate::types::user::AvatarDecorationData
#[derive(Debug)]
#[non_exhaustive]
pub struct AvatarDecorationDataSkuMarker;
