//! Builder patterns for ergonomic type construction.
//!
//! # Fork note
//!
//! Upstream twilight-model does not provide builders for its types. These are
//! our additions that make constructing `ApplicationCommand`, `Embed`, and
//! message components significantly more pleasant than filling in 15-field
//! structs with `None` everywhere.
//!
//! The component helper functions (`action_row`, `button`, `link_button`,
//! `string_select`, `text_input`) construct the twilight `Component` enum
//! variants directly, hiding the per-variant struct construction.

use crate::types::channel::message::{
    component::{
        ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType,
        TextInput, TextInputStyle,
    },
    embed::{Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedImage, EmbedThumbnail},
};

// ===========================================================================
// ApplicationCommand builder
// ===========================================================================

use crate::types::application::command::{
    Command, CommandOption, CommandOptionChoice, CommandOptionType, CommandType,
};
use crate::types::id::{marker::CommandMarker, Id};

/// Ergonomic builder for [`Command`] (aliased as `ApplicationCommand`).
///
/// # Examples
///
/// ```ignore
/// use crate::types::builders::ApplicationCommandBuilder;
///
/// let cmd = ApplicationCommandBuilder::chat_input("ping", "Check bot latency").build();
/// ```ignore
pub struct ApplicationCommandBuilder {
    inner: Command,
}

impl ApplicationCommandBuilder {
    /// Start building a CHAT_INPUT (slash) command.
    #[allow(deprecated)]
    pub fn chat_input(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            inner: Command {
                application_id: None,
                contexts: None,
                default_member_permissions: None,
                dm_permission: None,
                description: description.into(),
                description_localizations: None,
                guild_id: None,
                id: None,
                integration_types: None,
                kind: CommandType::ChatInput,
                name: name.into(),
                name_localizations: None,
                nsfw: None,
                options: Vec::new(),
                version: Id::new(1),
            },
        }
    }

    /// Start building a USER context-menu command.
    #[allow(dead_code)]
    #[allow(deprecated)]
    pub fn user(name: impl Into<String>) -> Self {
        Self {
            inner: Command {
                application_id: None,
                contexts: None,
                default_member_permissions: None,
                dm_permission: None,
                description: String::new(),
                description_localizations: None,
                guild_id: None,
                id: None,
                integration_types: None,
                kind: CommandType::User,
                name: name.into(),
                name_localizations: None,
                nsfw: None,
                options: Vec::new(),
                version: Id::new(1),
            },
        }
    }

    /// Start building a MESSAGE context-menu command.
    #[allow(dead_code)]
    #[allow(deprecated)]
    pub fn message(name: impl Into<String>) -> Self {
        Self {
            inner: Command {
                application_id: None,
                contexts: None,
                default_member_permissions: None,
                dm_permission: None,
                description: String::new(),
                description_localizations: None,
                guild_id: None,
                id: None,
                integration_types: None,
                kind: CommandType::Message,
                name: name.into(),
                name_localizations: None,
                nsfw: None,
                options: Vec::new(),
                version: Id::new(1),
            },
        }
    }

    /// Set the command ID (normally assigned by Discord, not needed for registration).
    #[allow(dead_code)]
    pub fn id(mut self, id: Id<CommandMarker>) -> Self {
        self.inner.id = Some(id);
        self
    }

    /// Add an option to the command.
    pub fn option(mut self, option: CommandOption) -> Self {
        self.inner.options.push(option);
        self
    }

    /// Add a simple option with just a name, description, type, and required flag.
    pub fn simple_option(
        mut self,
        kind: CommandOptionType,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.inner.options.push(CommandOption {
            autocomplete: None,
            channel_types: None,
            choices: None,
            description: description.into(),
            description_localizations: None,
            kind,
            max_length: None,
            max_value: None,
            min_length: None,
            min_value: None,
            name: name.into(),
            name_localizations: None,
            options: None,
            required: Some(required),
        });
        self
    }

    /// Mark the command as NSFW.
    #[allow(dead_code)]
    pub fn nsfw(mut self, nsfw: bool) -> Self {
        self.inner.nsfw = Some(nsfw);
        self
    }

    /// Consume the builder and return the finished [`Command`].
    pub fn build(self) -> Command {
        self.inner
    }
}

/// Convenience: build a [`CommandOption`] with choices.
#[allow(dead_code)]
pub fn command_option_with_choices(
    kind: CommandOptionType,
    name: impl Into<String>,
    description: impl Into<String>,
    required: bool,
    choices: Vec<CommandOptionChoice>,
) -> CommandOption {
    CommandOption {
        autocomplete: None,
        channel_types: None,
        choices: Some(choices),
        description: description.into(),
        description_localizations: None,
        kind,
        max_length: None,
        max_value: None,
        min_length: None,
        min_value: None,
        name: name.into(),
        name_localizations: None,
        options: None,
        required: Some(required),
    }
}

// ===========================================================================
// Embed builder
// ===========================================================================

/// Ergonomic builder for [`Embed`].
///
/// # Examples
///
/// ```ignore
/// use crate::types::builders::EmbedBuilder;
///
/// let embed = EmbedBuilder::new()
///     .title("Hello")
///     .description("World")
///     .color(0x00FF00)
///     .build();
/// ```ignore
pub struct EmbedBuilder {
    inner: Embed,
}

impl EmbedBuilder {
    /// Create a new empty embed builder.
    pub fn new() -> Self {
        Self {
            inner: Embed {
                author: None,
                color: None,
                description: None,
                fields: Vec::new(),
                footer: None,
                image: None,
                kind: String::new(),
                provider: None,
                thumbnail: None,
                timestamp: None,
                title: None,
                url: None,
                video: None,
            },
        }
    }

    /// Set the embed title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.inner.title = Some(title.into());
        self
    }

    /// Set the embed description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.inner.description = Some(desc.into());
        self
    }

    /// Set the embed color (as a 24-bit RGB integer, e.g. `0xFF6600`).
    pub fn color(mut self, color: u32) -> Self {
        self.inner.color = Some(color);
        self
    }

    /// Add a field to the embed.
    #[allow(dead_code)]
    pub fn field(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        inline: bool,
    ) -> Self {
        self.inner.fields.push(EmbedField {
            inline,
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Set the footer text.
    pub fn footer(mut self, text: impl Into<String>) -> Self {
        self.inner.footer = Some(EmbedFooter {
            icon_url: None,
            proxy_icon_url: None,
            text: text.into(),
        });
        self
    }

    /// Set the footer text and icon URL.
    #[allow(dead_code)]
    pub fn footer_with_icon(
        mut self,
        text: impl Into<String>,
        icon_url: impl Into<String>,
    ) -> Self {
        self.inner.footer = Some(EmbedFooter {
            icon_url: Some(icon_url.into()),
            proxy_icon_url: None,
            text: text.into(),
        });
        self
    }

    /// Set the thumbnail URL.
    #[allow(dead_code)]
    pub fn thumbnail(mut self, url: impl Into<String>) -> Self {
        self.inner.thumbnail = Some(EmbedThumbnail {
            height: None,
            proxy_url: None,
            url: url.into(),
            width: None,
        });
        self
    }

    /// Set the image URL.
    #[allow(dead_code)]
    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.inner.image = Some(EmbedImage {
            height: None,
            proxy_url: None,
            url: url.into(),
            width: None,
        });
        self
    }

    /// Set the embed author name.
    #[allow(dead_code)]
    pub fn author(mut self, name: impl Into<String>) -> Self {
        self.inner.author = Some(EmbedAuthor {
            icon_url: None,
            name: name.into(),
            proxy_icon_url: None,
            url: None,
        });
        self
    }

    /// Set the embed timestamp (ISO 8601 string).
    pub fn timestamp(mut self, ts: impl Into<String>) -> Self {
        // We store the timestamp as a `Timestamp` from our simplified datetime
        // module. If parsing fails, we silently skip.
        if let Ok(parsed) = crate::types::util::Timestamp::parse(&ts.into()) {
            self.inner.timestamp = Some(parsed);
        }
        self
    }

    /// Set the embed URL.
    #[allow(dead_code)]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.inner.url = Some(url.into());
        self
    }

    /// Consume the builder and return the finished [`Embed`].
    pub fn build(self) -> Embed {
        self.inner
    }
}

impl Default for EmbedBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Component helper functions
// ===========================================================================

/// Build an Action Row wrapping other components.
pub fn action_row(components: Vec<Component>) -> Component {
    Component::ActionRow(ActionRow {
        components,
        id: None,
    })
}

/// Build a button component.
///
/// `style` values: 1=Primary, 2=Secondary, 3=Success, 4=Danger.
/// For link buttons (style 5), use [`link_button`] instead.
pub fn button(style: u8, label: impl Into<String>, custom_id: impl Into<String>) -> Component {
    let button_style = match style {
        1 => ButtonStyle::Primary,
        2 => ButtonStyle::Secondary,
        3 => ButtonStyle::Success,
        4 => ButtonStyle::Danger,
        _ => ButtonStyle::Primary,
    };

    Component::Button(Button {
        custom_id: Some(custom_id.into()),
        disabled: false,
        emoji: None,
        label: Some(label.into()),
        style: button_style,
        url: None,
        sku_id: None,
        id: None,
    })
}

/// Build a link button (style 5, no custom_id, requires url).
#[allow(dead_code)]
pub fn link_button(label: impl Into<String>, url: impl Into<String>) -> Component {
    Component::Button(Button {
        custom_id: None,
        disabled: false,
        emoji: None,
        label: Some(label.into()),
        style: ButtonStyle::Link,
        url: Some(url.into()),
        sku_id: None,
        id: None,
    })
}

/// Build a string select menu component.
#[allow(dead_code)]
pub fn string_select(
    custom_id: impl Into<String>,
    placeholder: impl Into<String>,
    options: Vec<SelectMenuOption>,
) -> Component {
    Component::SelectMenu(SelectMenu {
        channel_types: None,
        custom_id: custom_id.into(),
        default_values: None,
        disabled: false,
        kind: SelectMenuType::Text,
        max_values: Some(1),
        min_values: Some(1),
        options: Some(options),
        placeholder: Some(placeholder.into()),
        id: None,
        required: None,
    })
}

/// Build a text input for use inside a modal.
///
/// `style`: 1 = Short, 2 = Paragraph.
pub fn text_input(
    custom_id: impl Into<String>,
    label: impl Into<String>,
    style: u8,
    required: bool,
) -> Component {
    let input_style = match style {
        1 => TextInputStyle::Short,
        2 => TextInputStyle::Paragraph,
        _ => TextInputStyle::Short,
    };

    #[allow(deprecated)]
    Component::TextInput(TextInput {
        custom_id: custom_id.into(),
        label: Some(label.into()),
        max_length: None,
        min_length: None,
        placeholder: None,
        required: Some(required),
        style: input_style,
        value: None,
        id: None,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_command_builder_chat_input() {
        let cmd = ApplicationCommandBuilder::chat_input("ping", "Check bot latency").build();
        assert_eq!(cmd.name, "ping");
        assert_eq!(cmd.description, "Check bot latency");
        assert!(matches!(cmd.kind, CommandType::ChatInput));
        assert!(cmd.options.is_empty());
    }

    #[test]
    fn application_command_builder_with_option() {
        let cmd = ApplicationCommandBuilder::chat_input("roll", "Roll a dice")
            .simple_option(
                CommandOptionType::Integer,
                "sides",
                "Number of sides",
                false,
            )
            .build();

        assert_eq!(cmd.options.len(), 1);
        assert_eq!(cmd.options[0].name, "sides");
        assert!(matches!(cmd.options[0].kind, CommandOptionType::Integer));
        assert_eq!(cmd.options[0].required, Some(false));
    }

    #[test]
    fn embed_builder_basic() {
        let embed = EmbedBuilder::new()
            .title("Test Title")
            .description("Test Description")
            .color(0xFF0000)
            .footer("Footer text")
            .build();

        assert_eq!(embed.title.as_deref(), Some("Test Title"));
        assert_eq!(embed.description.as_deref(), Some("Test Description"));
        assert_eq!(embed.color, Some(0xFF0000));
        assert!(embed.footer.is_some());
        assert_eq!(embed.footer.unwrap().text, "Footer text");
    }

    #[test]
    fn embed_builder_with_fields() {
        let embed = EmbedBuilder::new()
            .field("Name1", "Value1", true)
            .field("Name2", "Value2", false)
            .build();

        assert_eq!(embed.fields.len(), 2);
        assert_eq!(embed.fields[0].name, "Name1");
        assert!(embed.fields[0].inline);
        assert_eq!(embed.fields[1].name, "Name2");
        assert!(!embed.fields[1].inline);
    }

    #[test]
    fn action_row_wraps_components() {
        let row = action_row(vec![button(1, "Click", "btn_click")]);
        match row {
            Component::ActionRow(ar) => assert_eq!(ar.components.len(), 1),
            _ => panic!("expected ActionRow"),
        }
    }

    #[test]
    fn button_creates_correct_component() {
        let btn = button(3, "OK", "btn_ok");
        match btn {
            Component::Button(b) => {
                assert_eq!(b.label.as_deref(), Some("OK"));
                assert_eq!(b.custom_id.as_deref(), Some("btn_ok"));
                assert!(matches!(b.style, ButtonStyle::Success));
            }
            _ => panic!("expected Button"),
        }
    }

    #[test]
    fn link_button_has_url_and_no_custom_id() {
        let btn = link_button("Visit", "https://example.com");
        match btn {
            Component::Button(b) => {
                assert!(b.custom_id.is_none());
                assert_eq!(b.url.as_deref(), Some("https://example.com"));
                assert!(matches!(b.style, ButtonStyle::Link));
            }
            _ => panic!("expected Button"),
        }
    }

    #[test]
    fn text_input_creates_correct_component() {
        let ti = text_input("my_input", "Enter text", 2, true);
        match ti {
            Component::TextInput(t) => {
                assert_eq!(t.custom_id, "my_input");
                assert!(matches!(t.style, TextInputStyle::Paragraph));
                assert_eq!(t.required, Some(true));
            }
            _ => panic!("expected TextInput"),
        }
    }
}
