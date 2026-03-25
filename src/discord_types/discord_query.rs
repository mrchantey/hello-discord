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

		let mut info = MessageInfo::default();
		// Determine effective command text from @mention or ! prefix.
		if let Some(guild_id) = msg.guild_id
			&& bot_channels.get(&guild_id) == Some(&msg.channel_id)
		{
			info.bot_channel = true
		}
		if msg.mentions_user(bot_state.user_id()) {
			info.mentions_bot = true
		}
		info.xok()
	}
}

#[derive(Default)]
pub struct MessageInfo {
	bot_channel: bool,
	mentions_bot: bool,
}
impl MessageInfo {
	/// Returns true if the message is either sent in a bot channel
	/// or mentions the bot.
	pub fn is_direct_message(&self) -> bool {
		self.bot_channel || self.mentions_bot
	}
}
