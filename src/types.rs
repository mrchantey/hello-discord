//! Typed representations of Discord API objects.
//!
//! These mirror the Discord API docs so we can deserialize gateway events and
//! REST responses without touching `serde_json::Value` in the rest of the
//! codebase.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

/// Discord IDs are snowflakes transmitted as strings in JSON.
pub type Snowflake = String;

// ---------------------------------------------------------------------------
// Gateway payload (the envelope that wraps every WS message)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GatewayPayload {
    pub op: u8,
    pub d: Option<serde_json::Value>,
    pub s: Option<u64>,
    pub t: Option<String>,
}

// ---------------------------------------------------------------------------
// User
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub discriminator: Option<String>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
    pub global_name: Option<String>,
}

impl User {
    /// Returns the CDN URL for the user's avatar, or `None` if no avatar is set.
    pub fn avatar_url(&self) -> Option<String> {
        self.avatar.as_ref().map(|hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                self.id, hash
            )
        })
    }

    /// `Username#Discriminator` or just `Username` for the new username system.
    pub fn tag(&self) -> String {
        match self.discriminator.as_deref() {
            Some("0") | None => self.username.clone(),
            Some(disc) => format!("{}#{}", self.username, disc),
        }
    }
}

/// Partial user object (e.g. inside PRESENCE_UPDATE).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartialUser {
    pub id: Snowflake,
    pub username: Option<String>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

// ---------------------------------------------------------------------------
// Channel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ChannelType {
    GuildText = 0,
    Dm = 1,
    GuildVoice = 2,
    GroupDm = 3,
    GuildCategory = 4,
    GuildAnnouncement = 5,
    AnnouncementThread = 10,
    PublicThread = 11,
    PrivateThread = 12,
    GuildStageVoice = 13,
    GuildDirectory = 14,
    GuildForum = 15,
}

// We need serde_repr for enum-as-integer serialisation.
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Channel {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: ChannelType,
    pub guild_id: Option<Snowflake>,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub position: Option<i32>,
    pub parent_id: Option<Snowflake>,
    #[serde(default)]
    pub nsfw: bool,
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub author: User,
    pub content: String,
    pub timestamp: String,
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub tts: bool,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub pinned: bool,
    pub message_reference: Option<MessageReference>,
    /// The message this one is replying to (if resolved).
    pub referenced_message: Option<Box<Message>>,
    #[serde(default)]
    pub components: Vec<Component>,
    /// Interaction metadata when this message is an interaction response.
    pub interaction: Option<MessageInteraction>,
}

impl Message {
    /// Unix-millis timestamp derived from the message snowflake.
    pub fn snowflake_timestamp_ms(&self) -> Option<u64> {
        self.id
            .parse::<u64>()
            .ok()
            .map(|sf| (sf >> 22) + 1420070400000)
    }

    /// Whether a given user id is mentioned in the message.
    pub fn mentions_user(&self, user_id: &str) -> bool {
        self.mentions.iter().any(|u| u.id == user_id)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageReference {
    pub message_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    #[serde(default)]
    pub fail_if_not_exists: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageInteraction {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: u8,
    pub name: String,
    pub user: User,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    pub size: u64,
    pub url: String,
    pub proxy_url: String,
    pub content_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

// ---------------------------------------------------------------------------
// Embed
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

impl Embed {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    #[allow(dead_code)]
    pub fn field(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        inline: bool,
    ) -> Self {
        self.fields.push(EmbedField {
            name: name.into(),
            value: value.into(),
            inline,
        });
        self
    }

    pub fn footer(mut self, text: impl Into<String>) -> Self {
        self.footer = Some(EmbedFooter {
            text: text.into(),
            icon_url: None,
        });
        self
    }

    #[allow(dead_code)]
    pub fn footer_with_icon(
        mut self,
        text: impl Into<String>,
        icon_url: impl Into<String>,
    ) -> Self {
        self.footer = Some(EmbedFooter {
            text: text.into(),
            icon_url: Some(icon_url.into()),
        });
        self
    }

    #[allow(dead_code)]
    pub fn thumbnail(mut self, url: impl Into<String>) -> Self {
        self.thumbnail = Some(EmbedMedia { url: url.into() });
        self
    }

    #[allow(dead_code)]
    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image = Some(EmbedMedia { url: url.into() });
        self
    }

    #[allow(dead_code)]
    pub fn author(mut self, name: impl Into<String>) -> Self {
        self.author = Some(EmbedAuthor {
            name: name.into(),
            url: None,
            icon_url: None,
        });
        self
    }

    pub fn timestamp(mut self, ts: impl Into<String>) -> Self {
        self.timestamp = Some(ts.into());
        self
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbedFooter {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbedMedia {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbedAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

// ---------------------------------------------------------------------------
// Guild
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    pub icon: Option<String>,
    pub owner_id: Option<Snowflake>,
    pub approximate_member_count: Option<u64>,
    pub approximate_presence_count: Option<u64>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub members: Vec<GuildMember>,
}

impl Guild {
    /// Unix-millis timestamp derived from the guild snowflake.
    pub fn created_at_ms(&self) -> Option<u64> {
        self.id
            .parse::<u64>()
            .ok()
            .map(|sf| (sf >> 22) + 1420070400000)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnavailableGuild {
    pub id: Snowflake,
    #[serde(default)]
    pub unavailable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuildMember {
    pub user: Option<User>,
    pub nick: Option<String>,
    #[serde(default)]
    pub roles: Vec<Snowflake>,
    pub joined_at: Option<String>,
    #[serde(default)]
    pub deaf: bool,
    #[serde(default)]
    pub mute: bool,
}

// ---------------------------------------------------------------------------
// Presence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PresenceUpdate {
    pub user: PartialUser,
    pub guild_id: Option<Snowflake>,
    pub status: Option<String>,
    #[serde(default)]
    pub activities: Vec<Activity>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Activity {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: u8,
    pub url: Option<String>,
    pub state: Option<String>,
    pub details: Option<String>,
}

// ---------------------------------------------------------------------------
// READY event payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReadyEvent {
    pub v: u8,
    pub user: User,
    pub session_id: String,
    pub resume_gateway_url: String,
    #[serde(default)]
    pub guilds: Vec<UnavailableGuild>,
    pub application: ReadyApplication,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReadyApplication {
    pub id: Snowflake,
    pub flags: Option<u64>,
}

// ---------------------------------------------------------------------------
// Interactions (slash commands, buttons, select menus, modals)
// ---------------------------------------------------------------------------

/// Top-level interaction received via INTERACTION_CREATE.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Interaction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    #[serde(rename = "type")]
    pub kind: InteractionType,
    pub data: Option<InteractionData>,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub member: Option<GuildMember>,
    pub user: Option<User>,
    pub token: String,
    pub message: Option<Message>,
}

impl Interaction {
    /// Convenience: the user who triggered the interaction.
    pub fn author(&self) -> Option<&User> {
        self.member
            .as_ref()
            .and_then(|m| m.user.as_ref())
            .or(self.user.as_ref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum InteractionType {
    Ping = 1,
    ApplicationCommand = 2,
    MessageComponent = 3,
    ApplicationCommandAutocomplete = 4,
    ModalSubmit = 5,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InteractionData {
    /// Command / component ID.
    pub id: Option<Snowflake>,
    pub name: Option<String>,
    /// For components: the developer-defined `custom_id`.
    pub custom_id: Option<String>,
    /// Component type (for MESSAGE_COMPONENT interactions).
    pub component_type: Option<u8>,
    #[serde(default)]
    pub options: Vec<CommandOption>,
    /// Selected values from a select menu.
    #[serde(default)]
    pub values: Vec<String>,
    /// Modal submit components.
    #[serde(default)]
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandOption {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: u8,
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<CommandOption>,
    #[serde(default)]
    pub focused: bool,
}

// ---------------------------------------------------------------------------
// Interaction responses (what we send back)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub kind: InteractionCallbackType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<InteractionCallbackData>,
}

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

// Allow deserialising as well (useful in tests / echo scenarios)
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionCallbackData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// For modal responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Components (buttons, select menus, action rows, text inputs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Component {
    /// 1 = ActionRow, 2 = Button, 3 = StringSelect, 4 = TextInput,
    /// 5 = UserSelect, 6 = RoleSelect, 7 = MentionableSelect, 8 = ChannelSelect
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Button style: 1=Primary, 2=Secondary, 3=Success, 4=Danger, 5=Link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<SelectOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<ComponentEmoji>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<ComponentEmoji>,
    #[serde(default)]
    pub default: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComponentEmoji {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub animated: bool,
}

// ---------------------------------------------------------------------------
// Convenience builders for components
// ---------------------------------------------------------------------------

/// Build an Action Row wrapping other components.
pub fn action_row(components: Vec<Component>) -> Component {
    Component {
        kind: 1,
        custom_id: None,
        label: None,
        style: None,
        url: None,
        placeholder: None,
        min_values: None,
        max_values: None,
        min_length: None,
        max_length: None,
        required: None,
        value: None,
        options: Vec::new(),
        components,
        disabled: None,
        emoji: None,
    }
}

/// Build a button component.
pub fn button(style: u8, label: impl Into<String>, custom_id: impl Into<String>) -> Component {
    Component {
        kind: 2,
        custom_id: Some(custom_id.into()),
        label: Some(label.into()),
        style: Some(style),
        url: None,
        placeholder: None,
        min_values: None,
        max_values: None,
        min_length: None,
        max_length: None,
        required: None,
        value: None,
        options: Vec::new(),
        components: Vec::new(),
        disabled: None,
        emoji: None,
    }
}

/// Build a link button (style 5, no custom_id, requires url).
#[allow(dead_code)]
pub fn link_button(label: impl Into<String>, url: impl Into<String>) -> Component {
    Component {
        kind: 2,
        custom_id: None,
        label: Some(label.into()),
        style: Some(5),
        url: Some(url.into()),
        placeholder: None,
        min_values: None,
        max_values: None,
        min_length: None,
        max_length: None,
        required: None,
        value: None,
        options: Vec::new(),
        components: Vec::new(),
        disabled: None,
        emoji: None,
    }
}

/// Build a string select menu component.
#[allow(dead_code)]
pub fn string_select(
    custom_id: impl Into<String>,
    placeholder: impl Into<String>,
    options: Vec<SelectOption>,
) -> Component {
    Component {
        kind: 3,
        custom_id: Some(custom_id.into()),
        label: None,
        style: None,
        url: None,
        placeholder: Some(placeholder.into()),
        min_values: Some(1),
        max_values: Some(1),
        min_length: None,
        max_length: None,
        required: None,
        value: None,
        options,
        components: Vec::new(),
        disabled: None,
        emoji: None,
    }
}

/// Build a text input for use inside a modal.
pub fn text_input(
    custom_id: impl Into<String>,
    label: impl Into<String>,
    style: u8, // 1 = Short, 2 = Paragraph
    required: bool,
) -> Component {
    Component {
        kind: 4,
        custom_id: Some(custom_id.into()),
        label: Some(label.into()),
        style: Some(style),
        url: None,
        placeholder: None,
        min_values: None,
        max_values: None,
        min_length: None,
        max_length: None,
        required: Some(required),
        value: None,
        options: Vec::new(),
        components: Vec::new(),
        disabled: None,
        emoji: None,
    }
}

// ---------------------------------------------------------------------------
// Slash command registration payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommand {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ApplicationCommandOption>,
    /// 1 = CHAT_INPUT (slash), 2 = USER, 3 = MESSAGE
    #[serde(rename = "type", default = "default_command_type")]
    pub kind: u8,
}

fn default_command_type() -> u8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandOption {
    pub name: String,
    pub description: String,
    /// 1=SUB_COMMAND, 2=SUB_COMMAND_GROUP, 3=STRING, 4=INTEGER, 5=BOOLEAN,
    /// 6=USER, 7=CHANNEL, 8=ROLE, 9=MENTIONABLE, 10=NUMBER, 11=ATTACHMENT
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ApplicationCommandOptionChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandOptionChoice {
    pub name: String,
    pub value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Outbound message body (for REST POST /channels/{id}/messages)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

impl CreateMessage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(mut self, text: impl Into<String>) -> Self {
        self.content = Some(text.into());
        self
    }

    #[allow(dead_code)]
    pub fn embed(mut self, embed: Embed) -> Self {
        self.embeds.get_or_insert_with(Vec::new).push(embed);
        self
    }

    pub fn reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.message_reference = Some(MessageReference {
            message_id: Some(message_id.into()),
            channel_id: None,
            guild_id: None,
            fail_if_not_exists: false,
        });
        self
    }

    pub fn component_row(mut self, row: Component) -> Self {
        self.components.get_or_insert_with(Vec::new).push(row);
        self
    }
}

// ---------------------------------------------------------------------------
// Rate-limit info parsed from response headers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub remaining: Option<u32>,
    #[allow(dead_code)]
    pub reset_at: Option<f64>,
    pub reset_after: Option<f64>,
    pub bucket: Option<String>,
    pub is_global: bool,
}
