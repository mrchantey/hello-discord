//! Typed gateway events.
//!
//! Instead of matching on raw `(op, t, serde_json::Value)` tuples everywhere,
//! the gateway module deserialises dispatch payloads into this enum so the rest
//! of the bot can pattern-match on strongly-typed data.

use serde::Deserialize;
use tracing::warn;

use crate::types::*;

// ---------------------------------------------------------------------------
// The top-level event enum
// ---------------------------------------------------------------------------

/// A fully-parsed event coming off the Discord gateway.
#[derive(Debug, Clone)]
pub enum GatewayEvent {
    /// We've successfully identified / resumed — bot is ready.
    Ready(ReadyEvent),

    /// Full guild object lazily sent after READY.
    GuildCreate(Guild),

    /// A message was created in a channel we can see.
    MessageCreate(Message),

    /// A user's presence (online/idle/dnd/offline) changed.
    PresenceUpdate(PresenceUpdate),

    /// An interaction was created (slash command, button, select, modal submit).
    InteractionCreate(Interaction),

    /// Heartbeat ACK from the gateway (op 11).
    HeartbeatAck,

    /// The gateway is asking us to heartbeat immediately (op 1).
    HeartbeatRequest,

    /// Gateway told us to reconnect (op 7).
    Reconnect,

    /// Session has been invalidated (op 9). The inner bool indicates whether
    /// the session is resumable (`true`) or we must re-identify (`false`).
    InvalidSession(bool),

    /// An event we received but don't have a typed variant for yet.
    /// Carries the event name and raw JSON so callers can still inspect it.
    Unknown {
        event_name: Option<String>,
        #[allow(dead_code)]
        op: u8,
        #[allow(dead_code)]
        data: Option<serde_json::Value>,
    },
}

// ---------------------------------------------------------------------------
// Parsing from a raw GatewayPayload
// ---------------------------------------------------------------------------

impl GatewayEvent {
    /// Try to convert a raw [`GatewayPayload`] into a typed event.
    ///
    /// This never fails — unrecognised events become [`GatewayEvent::Unknown`].
    pub fn from_payload(payload: GatewayPayload) -> Self {
        match payload.op {
            // ----- Op 0: DISPATCH -----
            0 => Self::parse_dispatch(payload.t.as_deref(), payload.d),

            // ----- Op 1: Heartbeat request -----
            1 => GatewayEvent::HeartbeatRequest,

            // ----- Op 7: Reconnect -----
            7 => GatewayEvent::Reconnect,

            // ----- Op 9: Invalid Session -----
            9 => {
                let resumable = payload
                    .d
                    .as_ref()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                GatewayEvent::InvalidSession(resumable)
            }

            // ----- Op 11: Heartbeat ACK -----
            11 => GatewayEvent::HeartbeatAck,

            // ----- Anything else -----
            _ => GatewayEvent::Unknown {
                event_name: payload.t,
                op: payload.op,
                data: payload.d,
            },
        }
    }

    /// Parse an op-0 DISPATCH event by its `t` name.
    fn parse_dispatch(event_name: Option<&str>, data: Option<serde_json::Value>) -> Self {
        let Some(name) = event_name else {
            return GatewayEvent::Unknown {
                event_name: None,
                op: 0,
                data,
            };
        };

        let Some(d) = data else {
            return GatewayEvent::Unknown {
                event_name: Some(name.to_string()),
                op: 0,
                data: None,
            };
        };

        match name {
            "READY" => match serde_json::from_value::<ReadyEvent>(d.clone()) {
                Ok(ready) => GatewayEvent::Ready(ready),
                Err(e) => {
                    warn!(event = name, error = %e, "failed to parse READY payload");
                    GatewayEvent::Unknown {
                        event_name: Some(name.to_string()),
                        op: 0,
                        data: Some(d),
                    }
                }
            },

            "GUILD_CREATE" => match serde_json::from_value::<Guild>(d.clone()) {
                Ok(guild) => GatewayEvent::GuildCreate(guild),
                Err(e) => {
                    warn!(event = name, error = %e, "failed to parse GUILD_CREATE payload");
                    GatewayEvent::Unknown {
                        event_name: Some(name.to_string()),
                        op: 0,
                        data: Some(d),
                    }
                }
            },

            "MESSAGE_CREATE" => match serde_json::from_value::<Message>(d.clone()) {
                Ok(msg) => GatewayEvent::MessageCreate(msg),
                Err(e) => {
                    warn!(event = name, error = %e, "failed to parse MESSAGE_CREATE payload");
                    GatewayEvent::Unknown {
                        event_name: Some(name.to_string()),
                        op: 0,
                        data: Some(d),
                    }
                }
            },

            "PRESENCE_UPDATE" => match serde_json::from_value::<PresenceUpdate>(d.clone()) {
                Ok(presence) => GatewayEvent::PresenceUpdate(presence),
                Err(e) => {
                    warn!(event = name, error = %e, "failed to parse PRESENCE_UPDATE payload");
                    GatewayEvent::Unknown {
                        event_name: Some(name.to_string()),
                        op: 0,
                        data: Some(d),
                    }
                }
            },

            "INTERACTION_CREATE" => match serde_json::from_value::<Interaction>(d.clone()) {
                Ok(interaction) => GatewayEvent::InteractionCreate(interaction),
                Err(e) => {
                    warn!(event = name, error = %e, "failed to parse INTERACTION_CREATE payload");
                    GatewayEvent::Unknown {
                        event_name: Some(name.to_string()),
                        op: 0,
                        data: Some(d),
                    }
                }
            },

            // ---- Events we recognise but don't need typed variants for (yet) ----
            _ => GatewayEvent::Unknown {
                event_name: Some(name.to_string()),
                op: 0,
                data: Some(d),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convenience trait for pulling a typed value out of an Unknown event's data.
#[allow(dead_code)]
pub trait UnknownEventExt {
    /// If this is an `Unknown` event, try to deserialise its `data` field.
    fn try_parse_data<T: for<'de> Deserialize<'de>>(&self) -> Option<T>;
}

impl UnknownEventExt for GatewayEvent {
    fn try_parse_data<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        match self {
            GatewayEvent::Unknown { data: Some(d), .. } => serde_json::from_value(d.clone()).ok(),
            _ => None,
        }
    }
}
