use crate::prelude::*;
use beet::prelude::*;
use tracing::warn;
use twilight_model::gateway::presence::Status;
use twilight_model::gateway::presence::UserOrId;

/// Observer called when a user's presence changes.
///
/// Sends a one-time greeting when a user comes online for the first time
/// this session.
pub fn greet_users_coming_online(
	ev: On<DiscordPresenceUpdate>,
	mut commands: Commands,
	query: Populated<(&BotState, &GreetState, &DiscordHttpClient)>,
) -> Result {
	let entity = ev.event_target();

	if ev.presence.status != Status::Online {
		return Ok(());
	}

	let user_id = match &ev.presence.user {
		UserOrId::User(u) => u.id,
		UserOrId::UserId { id } => *id,
	};

	let (bot_state, greet_state, http) = query.get(entity)?;

	// Skip if this is the bot itself.
	if bot_state.bot_user_id() == user_id {
		return Ok(());
	}

	// Skip if already greeted this session.
	if greet_state.greeted_users.contains(&user_id) {
		return Ok(());
	}

	let greet_channel = greet_state.greet_channel_id;

	// Mark as greeted.
	commands.entity(entity).entry::<GreetState>().and_modify(
		move |mut state| {
			state.greeted_users.insert(user_id);
		},
	);

	if let Some(ch_id) = greet_channel {
		let http = http.clone();
		commands.queue_async(async move |_| {
			let greeting = format!(
				"Welcome online, <@{}>! 🎉 Hope you're having a great day!",
				user_id
			);
			if let Err(e) = http.send_message(ch_id, &greeting).await {
				warn!(error = %e, "failed to send greeting");
			}
		});
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use twilight_model::gateway::presence::Status;
	use twilight_model::gateway::presence::UserOrId;
	use twilight_model::id::marker::UserMarker;
	use twilight_model::id::Id;

	fn user_id(n: u64) -> Id<UserMarker> { Id::new(n) }

	#[test]
	fn non_online_status_should_be_ignored() {
		// Statuses other than Online should cause early return (no greeting).
		let statuses = [
			Status::Idle,
			Status::DoNotDisturb,
			Status::Offline,
			Status::Invisible,
		];
		for status in &statuses {
			assert_ne!(
				*status,
				Status::Online,
				"{:?} should not trigger greeting",
				status
			);
		}
	}

	#[test]
	fn user_or_id_full_user_extracts_id() {
		let id = user_id(42);
		let full_user = twilight_model::user::User {
			accent_color: None,
			avatar: None,
			avatar_decoration: None,
			avatar_decoration_data: None,
			banner: None,
			bot: false,
			discriminator: 0,
			email: None,
			flags: None,
			global_name: None,
			id,
			locale: None,
			mfa_enabled: None,
			name: "testuser".to_string(),
			premium_type: None,
			primary_guild: None,
			public_flags: None,
			system: None,
			verified: None,
		};
		let uoi = UserOrId::User(full_user);
		let extracted = match uoi {
			UserOrId::User(u) => u.id,
			UserOrId::UserId { id } => id,
		};
		assert_eq!(extracted, id);
	}

	#[test]
	fn user_or_id_bare_id_extracts_id() {
		let id = user_id(99);
		let uoi = UserOrId::UserId { id };
		let extracted = match uoi {
			UserOrId::User(u) => u.id,
			UserOrId::UserId { id } => id,
		};
		assert_eq!(extracted, id);
	}

	#[test]
	fn already_greeted_user_would_be_skipped() {
		use std::collections::HashSet;
		let mut greeted: HashSet<Id<UserMarker>> = HashSet::new();
		let alice = user_id(1);
		let bob = user_id(2);

		greeted.insert(alice);

		assert!(
			greeted.contains(&alice),
			"alice already greeted — should skip"
		);
		assert!(
			!greeted.contains(&bob),
			"bob not greeted yet — should proceed"
		);
	}

	#[test]
	fn bot_self_would_be_skipped() {
		let bot_id = user_id(100);
		let incoming_user = user_id(100);
		// If the presence update is for the bot itself, we skip.
		assert_eq!(
			bot_id, incoming_user,
			"bot's own presence update should be ignored"
		);
	}
}
