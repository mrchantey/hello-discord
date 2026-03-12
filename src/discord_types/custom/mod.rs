//! Types that don't exist in twilight-model.
//!
//! These are specific to our implementation and cover gateway envelopes,
//! outbound message bodies, rate-limit tracking, and READY event payloads
//! that twilight handles differently (via its own gateway crate).
mod misc;
pub use misc::*;
mod events;
pub use events::*;

pub use self::misc::{InteractionCallbackData, InteractionCallbackType, InteractionResponse};

// ---- Custom types (our additions) -----------------------------------------
pub use self::misc::{
    CreateMessage, GatewayPayload, PartialUser, PresenceUpdate, RateLimitInfo, ReadyApplication,
    ReadyEvent, UnknownEvent,
};
