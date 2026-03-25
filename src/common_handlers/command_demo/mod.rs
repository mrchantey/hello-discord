mod on_interaction;
mod parse_bang_command;
use crate::prelude::*;
use beet::prelude::*;
use on_interaction::*;
use parse_bang_command::*;
mod greet_state;
use greet_state::*;


/// Startup system that spawns the discord bot.
pub fn spawn_command_demo(mut commands: Commands) {
	commands
		.spawn((DiscordBot::default(), GreetState::default()))
		.observe(common_handlers::set_bot_state)
		.observe(parse_bang_command)
		.observe(handle_interaction);
}
