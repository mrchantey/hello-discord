//! Gateway (WebSocket) transport for the Discord API.
//!
//! This module owns the WebSocket connection lifecycle:
//!   - connect → receive HELLO → send IDENTIFY
//!   - background heartbeat task
//!   - sequence number + session_id tracking
//!   - automatic reconnect + RESUME on disconnect
//!   - gateway send rate limiting (120 events / 60s)
//!
//! The rest of the codebase consumes a stream of [`GatewayEvent`] values
//! without ever touching `tokio_tungstenite` directly — when we later swap
//! transports we only need to touch this file.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

use crate::events::GatewayEvent;
use crate::types::GatewayPayload;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

/// Discord allows at most 120 gateway sends per 60 seconds.
const SEND_BUDGET_MAX: u32 = 120;
const SEND_BUDGET_WINDOW: Duration = Duration::from_secs(60);

/// Maximum number of reconnect attempts before giving up for a while.
const MAX_RECONNECT_ATTEMPTS: u32 = 8;

// ---------------------------------------------------------------------------
// Gateway send rate limiter
// ---------------------------------------------------------------------------

/// Simple sliding-window rate limiter for outbound gateway messages.
struct SendRateLimiter {
    /// Timestamps of recent sends (ring buffer style — we just keep the
    /// window's worth).
    timestamps: Vec<Instant>,
    budget: u32,
    window: Duration,
}

impl SendRateLimiter {
    fn new(budget: u32, window: Duration) -> Self {
        Self {
            timestamps: Vec::with_capacity(budget as usize),
            budget,
            window,
        }
    }

    /// Returns how long the caller should wait before sending, or `None` if
    /// it can send immediately.  Does **not** record the send — call
    /// [`record`] after actually sending.
    fn delay(&self) -> Option<Duration> {
        if (self.timestamps.len() as u32) < self.budget {
            return None;
        }

        let now = Instant::now();
        // Prune timestamps outside the window conceptually — we just look at
        // how many are still inside.
        let in_window = self
            .timestamps
            .iter()
            .filter(|&&t| now.duration_since(t) < self.window)
            .count() as u32;

        if in_window < self.budget {
            return None;
        }

        // We're at capacity.  Find the oldest timestamp inside the window and
        // compute how long until it expires.
        let oldest_in_window = self
            .timestamps
            .iter()
            .filter(|&&t| now.duration_since(t) < self.window)
            .min()
            .copied();

        match oldest_in_window {
            Some(oldest) => {
                let expires_at = oldest + self.window;
                if expires_at > now {
                    Some(expires_at - now)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    /// Record a send at the current instant and prune old entries.
    fn record(&mut self) {
        let now = Instant::now();
        self.timestamps
            .retain(|&t| now.duration_since(t) < self.window);
        self.timestamps.push(now);
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Options for connecting to the Discord gateway.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub token: String,
    /// Gateway intents bitmask.
    pub intents: u32,
    /// Optional shard info: `[shard_id, num_shards]`.
    pub shard: Option<[u32; 2]>,
}

// ---------------------------------------------------------------------------
// Internal session state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct SessionState {
    /// From the READY event.
    session_id: Option<String>,
    /// Resume URL provided by Discord in the READY event.
    resume_gateway_url: Option<String>,
    /// Monotonically increasing sequence counter.
    sequence: Option<u64>,
}

// ---------------------------------------------------------------------------
// WebSocket writer wrapper (transport boundary)
// ---------------------------------------------------------------------------

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start the gateway connection and return a receiver of [`GatewayEvent`]s.
///
/// The returned `GatewayHandle` can be used to send messages on the gateway
/// (e.g. request guild members, update presence).  The background tasks will
/// keep running until the handle is dropped or an unrecoverable error occurs.
pub struct GatewayHandle {
    /// Send arbitrary JSON payloads on the gateway (rate-limited).
    #[allow(dead_code)]
    pub sender: mpsc::Sender<serde_json::Value>,
    /// Receive typed events.
    pub events: mpsc::Receiver<GatewayEvent>,
    /// Handle to the background driver task so callers can await / abort it.
    #[allow(dead_code)]
    pub driver_handle: tokio::task::JoinHandle<()>,
}

/// Connect to the Discord gateway, returning a [`GatewayHandle`].
///
/// This spawns background tasks for:
///   - reading from the WebSocket and parsing events
///   - heartbeating at the interval Discord tells us
///   - reconnecting + resuming on disconnects
///   - rate-limiting outbound sends
pub async fn connect(config: GatewayConfig) -> Result<GatewayHandle, String> {
    let (event_tx, event_rx) = mpsc::channel::<GatewayEvent>(256);
    let (send_tx, send_rx) = mpsc::channel::<serde_json::Value>(64);

    let driver_handle = tokio::spawn(gateway_driver(config, event_tx, send_rx));

    Ok(GatewayHandle {
        sender: send_tx,
        events: event_rx,
        driver_handle,
    })
}

// ---------------------------------------------------------------------------
// The main driver loop (runs in a spawned task)
// ---------------------------------------------------------------------------

async fn gateway_driver(
    config: GatewayConfig,
    event_tx: mpsc::Sender<GatewayEvent>,
    mut send_rx: mpsc::Receiver<serde_json::Value>,
) {
    let session = Arc::new(Mutex::new(SessionState::default()));
    let mut reconnect_attempts: u32 = 0;

    loop {
        // Decide which URL to connect to.
        let url = {
            let s = session.lock().await;
            s.resume_gateway_url
                .clone()
                .unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string())
        };

        // Append query params if the resume URL doesn't already have them.
        let url = if url.contains("v=10") {
            url
        } else if url.contains('?') {
            format!("{}&v=10&encoding=json", url)
        } else {
            format!("{}?v=10&encoding=json", url)
        };

        info!(url = %url, "connecting to Discord gateway");

        let ws_result = tokio_tungstenite::connect_async(&url).await;

        let (ws_stream, _) = match ws_result {
            Ok(pair) => {
                reconnect_attempts = 0;
                pair
            }
            Err(e) => {
                error!(error = %e, "failed to connect to gateway");
                reconnect_attempts += 1;
                if reconnect_attempts > MAX_RECONNECT_ATTEMPTS {
                    error!("exceeded max reconnect attempts, giving up");
                    return;
                }
                let backoff = backoff_delay(reconnect_attempts);
                warn!(
                    delay_ms = backoff.as_millis() as u64,
                    attempt = reconnect_attempts,
                    "backing off before reconnect"
                );
                tokio::time::sleep(backoff).await;
                continue;
            }
        };

        info!("WebSocket connected");

        let (ws_write, mut ws_read) = ws_stream.split();
        let ws_write = Arc::new(Mutex::new(ws_write));
        let rate_limiter = Arc::new(Mutex::new(SendRateLimiter::new(
            SEND_BUDGET_MAX,
            SEND_BUDGET_WINDOW,
        )));

        // ------------------------------------------------------------------
        // 1.  Read HELLO and extract heartbeat_interval
        // ------------------------------------------------------------------
        let heartbeat_interval = match read_hello_from_stream(&mut ws_read).await {
            Ok(interval) => interval,
            Err(e) => {
                error!(error = %e, "failed to read HELLO from gateway");
                reconnect_attempts += 1;
                let backoff = backoff_delay(reconnect_attempts);
                tokio::time::sleep(backoff).await;
                continue;
            }
        };

        info!(interval_ms = heartbeat_interval, "received HELLO");

        // ------------------------------------------------------------------
        // 2.  Send IDENTIFY or RESUME
        // ------------------------------------------------------------------
        let should_resume = {
            let s = session.lock().await;
            s.session_id.is_some() && s.sequence.is_some()
        };

        if should_resume {
            let s = session.lock().await;
            let resume = json!({
                "op": 6,
                "d": {
                    "token": config.token,
                    "session_id": s.session_id.as_ref().unwrap(),
                    "seq": s.sequence.unwrap(),
                }
            });
            drop(s);
            if let Err(e) = rate_limited_send(&ws_write, &rate_limiter, &resume).await {
                error!(error = %e, "failed to send RESUME");
                reconnect_attempts += 1;
                let backoff = backoff_delay(reconnect_attempts);
                tokio::time::sleep(backoff).await;
                continue;
            }
            info!("sent RESUME");
        } else {
            let mut identify = json!({
                "op": 2,
                "d": {
                    "token": config.token,
                    "properties": {
                        "os": "linux",
                        "browser": "rust-bot",
                        "device": "rust-bot"
                    },
                    "intents": config.intents,
                }
            });

            if let Some(shard) = config.shard {
                identify["d"]["shard"] = json!([shard[0], shard[1]]);
            }

            if let Err(e) = rate_limited_send(&ws_write, &rate_limiter, &identify).await {
                error!(error = %e, "failed to send IDENTIFY");
                reconnect_attempts += 1;
                let backoff = backoff_delay(reconnect_attempts);
                tokio::time::sleep(backoff).await;
                continue;
            }
            info!("sent IDENTIFY");
        }

        // ------------------------------------------------------------------
        // 3.  Spawn heartbeat task
        // ------------------------------------------------------------------
        let hb_write = Arc::clone(&ws_write);
        let hb_session = Arc::clone(&session);
        let hb_rate_limiter = Arc::clone(&rate_limiter);
        let (hb_cancel_tx, mut hb_cancel_rx) = mpsc::channel::<()>(1);

        let heartbeat_handle = tokio::spawn(async move {
            // Discord says we should send the first heartbeat after
            // `heartbeat_interval * jitter` where jitter ∈ [0, 1).
            let jitter = rand::random::<f64>();
            let first_delay = Duration::from_millis((heartbeat_interval as f64 * jitter) as u64);
            tokio::select! {
                _ = tokio::time::sleep(first_delay) => {}
                _ = hb_cancel_rx.recv() => { return; }
            }

            let mut interval = tokio::time::interval(Duration::from_millis(heartbeat_interval));
            // The first tick fires immediately; we already waited above.
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let seq = {
                            let s = hb_session.lock().await;
                            s.sequence
                        };
                        let heartbeat = json!({"op": 1, "d": seq});

                        if let Err(e) = rate_limited_send(&hb_write, &hb_rate_limiter, &heartbeat).await {
                            warn!(error = %e, "heartbeat send failed, stopping heartbeat task");
                            return;
                        }
                        debug!("sent heartbeat (seq={:?})", seq);
                    }
                    _ = hb_cancel_rx.recv() => {
                        debug!("heartbeat task cancelled");
                        return;
                    }
                }
            }
        });

        // ------------------------------------------------------------------
        // 4.  Main read loop
        // ------------------------------------------------------------------
        let disconnect_reason = read_loop(
            &mut ws_read,
            &ws_write,
            &rate_limiter,
            &event_tx,
            &session,
            &config,
            &mut send_rx,
        )
        .await;

        // ------------------------------------------------------------------
        // 5.  Cleanup — cancel heartbeat, decide whether to reconnect
        // ------------------------------------------------------------------
        let _ = hb_cancel_tx.send(()).await;
        heartbeat_handle.abort();

        // Try to close the WS gracefully.
        {
            let mut w = ws_write.lock().await;
            let _ = w
                .send(tokio_tungstenite::tungstenite::Message::Close(None))
                .await;
        }

        match disconnect_reason {
            DisconnectReason::ShouldResume => {
                info!("will attempt RESUME");
                // session state is preserved; loop will send RESUME.
            }
            DisconnectReason::ShouldReidentify => {
                info!("session invalidated, will re-IDENTIFY");
                let mut s = session.lock().await;
                s.session_id = None;
                s.sequence = None;
                // Keep resume_gateway_url for the next attempt.
            }
            DisconnectReason::Fatal => {
                error!("fatal gateway error, shutting down");
                return;
            }
            DisconnectReason::EventChannelClosed => {
                info!("event channel closed, shutting down gateway driver");
                return;
            }
        }

        reconnect_attempts += 1;
        if reconnect_attempts > MAX_RECONNECT_ATTEMPTS {
            error!("exceeded max reconnect attempts, giving up");
            return;
        }
        let backoff = backoff_delay(reconnect_attempts);
        warn!(
            delay_ms = backoff.as_millis() as u64,
            attempt = reconnect_attempts,
            "reconnecting after backoff"
        );
        tokio::time::sleep(backoff).await;
    }
}

// ---------------------------------------------------------------------------
// Disconnect reason
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum DisconnectReason {
    ShouldResume,
    ShouldReidentify,
    Fatal,
    EventChannelClosed,
}

// ---------------------------------------------------------------------------
// Read loop
// ---------------------------------------------------------------------------

async fn read_loop(
    ws_read: &mut WsStream,
    ws_write: &Arc<Mutex<WsSink>>,
    rate_limiter: &Arc<Mutex<SendRateLimiter>>,
    event_tx: &mpsc::Sender<GatewayEvent>,
    session: &Arc<Mutex<SessionState>>,
    _config: &GatewayConfig,
    send_rx: &mut mpsc::Receiver<serde_json::Value>,
) -> DisconnectReason {
    loop {
        tokio::select! {
            biased;

            // Outbound sends from the bot logic (e.g. update presence).
            Some(payload) = send_rx.recv() => {
                if let Err(e) = rate_limited_send(ws_write, rate_limiter, &payload).await {
                    warn!(error = %e, "failed to send user payload on gateway");
                }
            }

            // Inbound messages from Discord.
            msg = ws_read.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        warn!(error = %e, "WebSocket read error");
                        return DisconnectReason::ShouldResume;
                    }
                    None => {
                        info!("WebSocket stream ended");
                        return DisconnectReason::ShouldResume;
                    }
                };

                match msg {
                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                        let payload: GatewayPayload = match serde_json::from_str(&text) {
                            Ok(p) => p,
                            Err(e) => {
                                warn!(error = %e, "failed to parse gateway payload");
                                continue;
                            }
                        };

                        // Update sequence number.
                        if let Some(s) = payload.s {
                            let mut sess = session.lock().await;
                            sess.sequence = Some(s);
                        }

                        let event = GatewayEvent::from_payload(payload);

                        // Handle session-relevant events internally.
                        match &event {
                            GatewayEvent::Ready(ready) => {
                                let mut sess = session.lock().await;
                                sess.session_id = Some(ready.session_id.clone());
                                sess.resume_gateway_url = Some(ready.resume_gateway_url.clone());
                                info!(
                                    session_id = %ready.session_id,
                                    user = %ready.user.username,
                                    "gateway READY"
                                );
                            }

                            GatewayEvent::HeartbeatRequest => {
                                // Respond with an immediate heartbeat.
                                let seq = {
                                    let s = session.lock().await;
                                    s.sequence
                                };
                                let heartbeat = json!({"op": 1, "d": seq});
                                if let Err(e) = rate_limited_send(ws_write, rate_limiter, &heartbeat).await {
                                    warn!(error = %e, "failed to send requested heartbeat");
                                }
                                debug!("sent requested heartbeat");
                                // Don't forward to bot — it's internal plumbing.
                                continue;
                            }

                            GatewayEvent::HeartbeatAck => {
                                debug!("heartbeat acknowledged");
                            }

                            GatewayEvent::Reconnect => {
                                info!("gateway requested reconnect (op 7)");
                                return DisconnectReason::ShouldResume;
                            }

                            GatewayEvent::InvalidSession(resumable) => {
                                warn!(resumable, "session invalidated (op 9)");
                                if *resumable {
                                    // Wait a bit, then resume.
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    return DisconnectReason::ShouldResume;
                                } else {
                                    tokio::time::sleep(Duration::from_secs(3)).await;
                                    return DisconnectReason::ShouldReidentify;
                                }
                            }

                            _ => {}
                        }

                        // Forward to bot.
                        if event_tx.send(event).await.is_err() {
                            info!("event channel closed by consumer");
                            return DisconnectReason::EventChannelClosed;
                        }
                    }

                    tokio_tungstenite::tungstenite::Message::Close(frame) => {
                        let code = frame.as_ref().map(|f| f.code);
                        warn!(close_code = ?code, "WebSocket closed by server");

                        // Certain close codes are fatal (authentication failed,
                        // invalid intents, etc.).
                        if let Some(frame) = &frame {
                            let raw: u16 = frame.code.into();
                            match raw {
                                4004 => {
                                    error!("authentication failed (close 4004)");
                                    return DisconnectReason::Fatal;
                                }
                                4010 => {
                                    error!("invalid shard (close 4010)");
                                    return DisconnectReason::Fatal;
                                }
                                4011 => {
                                    error!("sharding required (close 4011)");
                                    return DisconnectReason::Fatal;
                                }
                                4012 => {
                                    error!("invalid API version (close 4012)");
                                    return DisconnectReason::Fatal;
                                }
                                4013 => {
                                    error!("invalid intents (close 4013)");
                                    return DisconnectReason::Fatal;
                                }
                                4014 => {
                                    error!("disallowed intents (close 4014)");
                                    return DisconnectReason::Fatal;
                                }
                                4007 | 4009 => {
                                    // Invalid seq or session timed out — re-identify.
                                    return DisconnectReason::ShouldReidentify;
                                }
                                _ => {
                                    // Everything else: try to resume.
                                    return DisconnectReason::ShouldResume;
                                }
                            }
                        }

                        return DisconnectReason::ShouldResume;
                    }

                    // Ping/Pong/Binary — ignore.
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read the HELLO payload from an already-split stream reference.
async fn read_hello_from_stream(stream: &mut WsStream) -> Result<u64, String> {
    let msg = tokio::time::timeout(Duration::from_secs(30), stream.next())
        .await
        .map_err(|_| "timed out waiting for HELLO".to_string())?
        .ok_or_else(|| "stream ended before HELLO".to_string())?
        .map_err(|e| format!("WS error reading HELLO: {}", e))?;

    let text = match msg {
        tokio_tungstenite::tungstenite::Message::Text(t) => t,
        other => return Err(format!("expected text message for HELLO, got {:?}", other)),
    };

    let payload: GatewayPayload =
        serde_json::from_str(&text).map_err(|e| format!("failed to parse HELLO: {}", e))?;

    if payload.op != 10 {
        return Err(format!("expected op 10 (HELLO), got op {}", payload.op));
    }

    let interval = payload
        .d
        .as_ref()
        .and_then(|d| d.get("heartbeat_interval"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "HELLO missing heartbeat_interval".to_string())?;

    Ok(interval)
}

/// Send a JSON payload on the WebSocket, respecting the send rate limiter.
async fn rate_limited_send(
    ws_write: &Arc<Mutex<WsSink>>,
    rate_limiter: &Arc<Mutex<SendRateLimiter>>,
    payload: &serde_json::Value,
) -> Result<(), String> {
    // Wait until we have budget.
    loop {
        let delay = {
            let rl = rate_limiter.lock().await;
            rl.delay()
        };
        match delay {
            Some(d) => {
                debug!(
                    delay_ms = d.as_millis() as u64,
                    "gateway send rate-limited, waiting"
                );
                tokio::time::sleep(d).await;
            }
            None => break,
        }
    }

    // Record the send.
    {
        let mut rl = rate_limiter.lock().await;
        rl.record();
    }

    let text = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    let mut w = ws_write.lock().await;
    w.send(tokio_tungstenite::tungstenite::Message::Text(text))
        .await
        .map_err(|e| format!("WS send error: {}", e))
}

/// Exponential backoff with jitter, capped at 60 s.
fn backoff_delay(attempt: u32) -> Duration {
    let base_ms = 1000u64 * 2u64.saturating_pow(attempt.min(6));
    let jitter = (rand::random::<f64>() * 0.5 + 0.75) * base_ms as f64;
    Duration::from_millis(jitter.min(60_000.0) as u64)
}
