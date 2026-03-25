use crate::prelude::*;
use beet::prelude::*;

#[derive(SystemParam)]
pub struct DiscordQuery<'w, 's> {
	pub bots: Query<'w, 's, (&'static BotState, &'static BotChannels)>,
}

impl<'w, 's> DiscordQuery<'w, 's> {
	pub fn message_info(&self, ev: &DiscordMessage) -> Result<MessageInfo> {
		let entity = ev.event_target();
		let msg = &ev.message;
		let (bot_state, bot_channels) = self.bots.get(entity)?;

		MessageInfo {
			is_bot: ev.author.bot,
			is_self: ev.author.id == bot_state.user_id(),
			bot_channel: msg.guild_id.map_or(false, |guild_id| {
				bot_channels.get(&guild_id) == Some(&msg.channel_id)
			}),
			mentions_bot: msg.mentions_user(bot_state.user_id()),
		}
		.xok()
	}
}

#[derive(Default)]
pub struct MessageInfo {
	is_bot: bool,
	is_self: bool,
	bot_channel: bool,
	mentions_bot: bool,
}
impl MessageInfo {
	/// Returns true if the message is either sent in a bot channel
	/// or mentions the bot.
	pub fn is_direct_message(&self) -> bool {
		if self.is_self {
			false
		} else if self.bot_channel {
			true
		} else if self.mentions_bot {
			true
		} else {
			false
		}
	}
	pub fn is_bot(&self) -> bool { self.is_bot }
}
