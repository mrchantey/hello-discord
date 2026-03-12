//! Lightweight gateway types borrowed from `twilight-gateway`.
//!
//! We can't depend on `twilight-gateway` directly (it pulls in `tokio`,
//! `tokio-websockets`, etc.), but we *can* re-export the pure data types
//! from `twilight-model` and adapt a few patterns from twilight-gateway
//! that only depend on those model types.
//!
//! Everything here is intentionally thin — just types, constants, and
//! conversions. The actual WebSocket transport lives in
//! [`crate::discord_io::gateway`].

// =========================================================================
// Re-exports from twilight-model::gateway
// =========================================================================

pub use twilight_model::gateway::CloseCode;
pub use twilight_model::gateway::CloseFrame;
pub use twilight_model::gateway::Intents;
pub use twilight_model::gateway::OpCode;

// =========================================================================
// Gateway payload envelope
// =========================================================================

use serde::{Deserialize, Serialize};

/// Raw gateway payload envelope.
///
/// Every message on the Discord WebSocket is wrapped in this structure.
/// Twilight handles this inside `twilight-gateway`, but since we run our
/// own transport we need the raw envelope.  The `op` field is now the
/// strongly-typed [`OpCode`] instead of a bare `u8`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GatewayPayload {
    pub op: OpCode,
    pub d: Option<serde_json::Value>,
    pub s: Option<u64>,
    pub t: Option<String>,
}

// =========================================================================
// Close-code helpers
// =========================================================================

/// Classifies a raw WebSocket close code into an action the gateway
/// driver should take.
///
/// Uses [`CloseCode`] from twilight-model where possible, falling back
/// to a sensible default for codes Discord doesn't officially define.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Session is still alive — reconnect and send RESUME.
    Resume,
    /// Session was invalidated — reconnect and send IDENTIFY.
    Reidentify,
    /// Unrecoverable error — stop the gateway driver.
    Fatal,
}

impl CloseAction {
    /// Determine the appropriate action for a raw close code received
    /// from Discord.
    pub fn from_code(raw: u16) -> Self {
        match CloseCode::try_from(raw) {
            Ok(code) => {
                if code.can_reconnect() {
                    // InvalidSequence and SessionTimedOut mean the session
                    // is gone — we need to re-IDENTIFY rather than RESUME.
                    match code {
                        CloseCode::InvalidSequence | CloseCode::SessionTimedOut => Self::Reidentify,
                        _ => Self::Resume,
                    }
                } else {
                    Self::Fatal
                }
            }
            // Unknown code — try to resume as a safe default.
            Err(_) => Self::Resume,
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_payload_deserializes() {
        let json = r#"{"op":0,"d":null,"s":1,"t":"READY"}"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OpCode::Dispatch);
        assert_eq!(payload.s, Some(1));
        assert_eq!(payload.t.as_deref(), Some("READY"));
    }

    #[test]
    fn gateway_payload_hello() {
        let json = r#"{"op":10,"d":{"heartbeat_interval":41250},"s":null,"t":null}"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OpCode::Hello);
    }

    #[test]
    fn close_action_fatal_codes() {
        assert_eq!(CloseAction::from_code(4004), CloseAction::Fatal);
        assert_eq!(CloseAction::from_code(4010), CloseAction::Fatal);
        assert_eq!(CloseAction::from_code(4011), CloseAction::Fatal);
        assert_eq!(CloseAction::from_code(4012), CloseAction::Fatal);
        assert_eq!(CloseAction::from_code(4013), CloseAction::Fatal);
        assert_eq!(CloseAction::from_code(4014), CloseAction::Fatal);
    }

    #[test]
    fn close_action_reidentify_codes() {
        assert_eq!(CloseAction::from_code(4007), CloseAction::Reidentify);
        assert_eq!(CloseAction::from_code(4009), CloseAction::Reidentify);
    }

    #[test]
    fn close_action_resume_codes() {
        assert_eq!(CloseAction::from_code(4000), CloseAction::Resume);
        assert_eq!(CloseAction::from_code(4001), CloseAction::Resume);
        assert_eq!(CloseAction::from_code(4002), CloseAction::Resume);
        assert_eq!(CloseAction::from_code(4003), CloseAction::Resume);
        assert_eq!(CloseAction::from_code(4005), CloseAction::Resume);
        assert_eq!(CloseAction::from_code(4008), CloseAction::Resume);
    }

    #[test]
    fn close_action_unknown_code_resumes() {
        assert_eq!(CloseAction::from_code(5000), CloseAction::Resume);
        assert_eq!(CloseAction::from_code(1001), CloseAction::Resume);
    }

    #[test]
    fn intents_bitflags() {
        let intents = Intents::GUILDS
            | Intents::GUILD_MEMBERS
            | Intents::GUILD_PRESENCES
            | Intents::GUILD_MESSAGES
            | Intents::MESSAGE_CONTENT;
        assert!(intents.contains(Intents::GUILDS));
        assert!(intents.contains(Intents::GUILD_MEMBERS));
        assert!(intents.contains(Intents::GUILD_PRESENCES));
        assert!(intents.contains(Intents::GUILD_MESSAGES));
        assert!(intents.contains(Intents::MESSAGE_CONTENT));
        assert!(!intents.contains(Intents::DIRECT_MESSAGES));
    }
}
