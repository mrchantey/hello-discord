use beet::prelude::*;
use twilight_model::gateway::payload::incoming::Ready;

#[derive(Debug, Clone, EntityEvent)]
pub struct DiscordReady {
	entity: Entity,
	ready: Ready,
}

impl DiscordReady {
	pub fn create(ready: Ready) -> impl FnOnce(Entity) -> Self {
		move |entity| Self { entity, ready }
	}
}
