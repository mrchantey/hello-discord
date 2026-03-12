//! Discord I/O layer: gateway (WebSocket) transport, HTTP REST client,
//! event-handlers, and the Bevy bot wiring.
//!
//! This entire module is compiled only when the `io` feature is enabled,
//! keeping the default build to just [`crate::discord_helpers`].

pub mod gateway;
pub mod gateway_listener;
pub mod handlers;
pub mod http;
