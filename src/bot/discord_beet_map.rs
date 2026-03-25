#![allow(unused)]
use crate::prelude::*;
use beet::prelude::*;
use twilight_model::guild::Guild;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

// TODO
type ThreadId = u32;
type ActorId = u32;


/// Map a discord channel to a beet thread,
/// including top level channels and 'discord threads'
pub struct DiscordThread {
	thread: ThreadId,
	guild: Id<Guild>,
	/// May be a discord 'thread'
	channel: Id<ChannelMarker>,
}


/// Map a discord user to a beet actor,
/// including humans and 'discord bots'.
pub struct DiscordActor {
	actor: ActorId,
	user: Id<UserMarker>,
}
