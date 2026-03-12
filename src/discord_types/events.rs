use beet::prelude::*;

#[derive(Debug, Clone, EntityEvent)]
pub struct Ready {
	entity: Entity,
}
