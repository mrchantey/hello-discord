//! Transport-abstracted HTTP client for the Discord REST API.
//!
//! All outbound HTTP calls go through [`DiscordHttpClient`] so that auth
//! headers, rate-limit back-off, and error handling live in one place.
//! The beet `Request` / `Response` types are an implementation detail —
//! swapping HTTP backends only requires touching this module.

use async_lock::Mutex;
use beet::core::time_ext;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use beet::core::prelude::{HttpMethod, Request, ResponseParts, StatusCode};
use beet::net::prelude::RequestClientExt;

use crate::types::*;
use serde_json::json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const BASE_URL: &str = "https://discord.com/api/v10";
const USER_AGENT: &str = "BeetFramework (https://github.com/mrchantey/beet, 0.1)";

// ---------------------------------------------------------------------------
// Rate-limit tracker (per-bucket)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BucketState {
    remaining: u32,
    resets_at: Instant,
}

#[derive(Debug, Clone)]
struct RateLimiter {
    /// Route-key → bucket id mapping.
    route_buckets: HashMap<String, String>,
    /// Bucket id → state.
    buckets: HashMap<String, BucketState>,
    /// Global rate-limit: if set, no requests may be sent until this instant.
    global_until: Option<Instant>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            route_buckets: HashMap::new(),
            buckets: HashMap::new(),
            global_until: None,
        }
    }

    /// Returns how long we should wait before sending a request on `route_key`,
    /// or `None` if we can send immediately.
    fn delay_for(&self, route_key: &str) -> Option<Duration> {
        // Global rate limit takes priority.
        if let Some(until) = self.global_until {
            let now = Instant::now();
            if until > now {
                return Some(until - now);
            }
        }

        let bucket_id = self.route_buckets.get(route_key)?;
        let state = self.buckets.get(bucket_id)?;

        if state.remaining == 0 {
            let now = Instant::now();
            if state.resets_at > now {
                return Some(state.resets_at - now);
            }
        }

        None
    }

    /// Update internal state from response headers.
    fn update(&mut self, route_key: &str, info: &RateLimitInfo) {
        if info.is_global {
            if let Some(reset_after) = info.reset_after {
                self.global_until = Some(Instant::now() + Duration::from_secs_f64(reset_after));
            }
        }

        if let Some(ref bucket) = info.bucket {
            self.route_buckets
                .insert(route_key.to_string(), bucket.clone());

            let reset_instant = if let Some(reset_after) = info.reset_after {
                Instant::now() + Duration::from_secs_f64(reset_after)
            } else {
                Instant::now() + Duration::from_secs(1)
            };

            self.buckets.insert(
                bucket.clone(),
                BucketState {
                    remaining: info.remaining.unwrap_or(1),
                    resets_at: reset_instant,
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Parse rate-limit headers from response parts
// ---------------------------------------------------------------------------

fn parse_rate_limit_headers(parts: &ResponseParts) -> RateLimitInfo {
    let remaining = parts
        .get_header("x-ratelimit-remaining")
        .and_then(|s: &str| s.parse::<u32>().ok());

    let reset_at = parts
        .get_header("x-ratelimit-reset")
        .and_then(|s: &str| s.parse::<f64>().ok());

    let reset_after = parts
        .get_header("x-ratelimit-reset-after")
        .and_then(|s: &str| s.parse::<f64>().ok());

    let bucket = parts
        .get_header("x-ratelimit-bucket")
        .map(|s: &str| s.to_string());

    let is_global = parts
        .get_header("x-ratelimit-global")
        .map(|s: &str| s == "true")
        .unwrap_or(false);

    RateLimitInfo {
        remaining,
        reset_at,
        reset_after,
        bucket,
        is_global,
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum HttpError {
    /// Non-success status from Discord.
    Api {
        status: u16,
        body: String,
        route: String,
    },
    /// Transport / network error.
    Transport(String),
    /// Serialisation error.
    Serde(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Api {
                status,
                body,
                route,
            } => {
                write!(f, "Discord API error {} on {}: {}", status, route, body)
            }
            HttpError::Transport(e) => write!(f, "HTTP transport error: {}", e),
            HttpError::Serde(e) => write!(f, "Serialisation error: {}", e),
        }
    }
}

impl std::error::Error for HttpError {}

// ---------------------------------------------------------------------------
// DiscordHttpClient
// ---------------------------------------------------------------------------

/// A thin, rate-limit–aware HTTP client for the Discord REST API.
///
/// Cheap to clone (internals are behind `Arc`).
#[derive(Clone)]
pub struct DiscordHttpClient {
    token: String,
    limiter: Arc<Mutex<RateLimiter>>,
}

impl DiscordHttpClient {
    /// Create a new client with the given bot token.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            limiter: Arc::new(Mutex::new(RateLimiter::new())),
        }
    }

    // ------------------------------------------------------------------
    // Internal helper: build a base Request with auth + user-agent
    // ------------------------------------------------------------------

    fn build_request(&self, method: HttpMethod, url: &str) -> Request {
        let mut req = Request::new(method, url);
        req.insert_header("authorization", format!("Bot {}", self.token));
        req.insert_header("user-agent", USER_AGENT);
        req
    }

    // ------------------------------------------------------------------
    // Low-level: the single request method everything funnels through
    // ------------------------------------------------------------------

    /// Send a request to `{BASE_URL}/{path}`.
    ///
    /// `route_key` is used for per-route rate-limit bucketing. It should be a
    /// template like `POST /channels/{channel_id}/messages`.
    ///
    /// Returns the raw response body as bytes on success.
    pub async fn request(
        &self,
        method: HttpMethod,
        path: &str,
        route_key: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Vec<u8>, HttpError> {
        let max_retries = 5;
        for attempt in 0..=max_retries {
            // Pre-request: wait if the rate limiter says so.
            {
                let limiter = self.limiter.lock().await;
                if let Some(delay) = limiter.delay_for(route_key) {
                    let delay = delay.min(Duration::from_secs(60));
                    drop(limiter);
                    debug!(
                        route = route_key,
                        delay_ms = delay.as_millis() as u64,
                        "rate-limit pre-emptive backoff"
                    );
                    time_ext::sleep(delay).await;
                }
            }

            let url = format!("{}/{}", BASE_URL, path.trim_start_matches('/'));

            let req = self.build_request(method, &url);
            let req = if let Some(json) = body {
                req.with_json_body(json)
                    .map_err(|e| HttpError::Serde(e.to_string()))?
            } else {
                req
            };

            let resp = req
                .send()
                .await
                .map_err(|e| HttpError::Transport(e.to_string()))?;

            let status = resp.status();
            let rl_info = parse_rate_limit_headers(resp.response_parts());

            // Update the limiter regardless of status.
            {
                let mut limiter = self.limiter.lock().await;
                limiter.update(route_key, &rl_info);
            }

            if status == StatusCode::RateLimitExceeded {
                let retry_after = rl_info.reset_after.unwrap_or(1.0);
                let delay = Duration::from_secs_f64(retry_after.min(60.0));
                warn!(
                    route = route_key,
                    attempt,
                    retry_after_s = retry_after,
                    global = rl_info.is_global,
                    "rate-limited by Discord, backing off"
                );

                if rl_info.is_global {
                    let mut limiter = self.limiter.lock().await;
                    limiter.global_until = Some(Instant::now() + delay);
                }

                if attempt < max_retries {
                    time_ext::sleep(delay).await;
                    continue;
                }
            }

            let resp_bytes = resp
                .bytes()
                .await
                .map_err(|e: beet::core::prelude::BevyError| HttpError::Transport(e.to_string()))?;

            if status.is_ok() {
                return Ok(resp_bytes.to_vec());
            }

            let status_u16 = status_to_u16(status);
            let body_str = String::from_utf8_lossy(&resp_bytes).to_string();
            return Err(HttpError::Api {
                status: status_u16,
                body: body_str,
                route: route_key.to_string(),
            });
        }

        Err(HttpError::Api {
            status: 429,
            body: "rate-limited after max retries".to_string(),
            route: route_key.to_string(),
        })
    }

    /// Like [`request`] but deserialises the response body as JSON.
    pub async fn request_json<T: serde::de::DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        route_key: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T, HttpError> {
        let bytes = self.request(method, path, route_key, body).await?;
        serde_json::from_slice(&bytes).map_err(|e| {
            let raw = String::from_utf8_lossy(&bytes);
            HttpError::Serde(format!("{}: {}", e, &raw[..raw.len().min(200)]))
        })
    }

    // ------------------------------------------------------------------
    // Convenience: Messages
    // ------------------------------------------------------------------

    /// Send a simple text message to a channel.
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<Message, HttpError> {
        let msg = CreateMessage::new().content(content);
        self.create_message(channel_id, &msg).await
    }

    /// Send a rich message (embeds, components, reply, etc.) to a channel.
    pub async fn create_message(
        &self,
        channel_id: &str,
        msg: &CreateMessage,
    ) -> Result<Message, HttpError> {
        let path = format!("channels/{}/messages", channel_id);
        let route_key = format!("POST /channels/{}/messages", channel_id);
        let body = serde_json::to_value(msg).map_err(|e| HttpError::Serde(e.to_string()))?;
        self.request_json(HttpMethod::Post, &path, &route_key, Some(&body))
            .await
    }

    /// Send a message with a file attachment to a channel.
    pub async fn send_message_with_file(
        &self,
        channel_id: &str,
        content: Option<&str>,
        filename: &str,
        file_content: Vec<u8>,
    ) -> Result<Message, HttpError> {
        let path = format!("channels/{}/messages", channel_id);
        let route_key = format!("POST /channels/{}/messages", channel_id);
        let url = format!("{}/{}", BASE_URL, path.trim_start_matches('/'));

        // Pre-request rate-limit wait.
        {
            let limiter = self.limiter.lock().await;
            if let Some(delay) = limiter.delay_for(&route_key) {
                let delay = delay.min(Duration::from_secs(60));
                drop(limiter);
                debug!(
                    route = route_key,
                    delay_ms = delay.as_millis() as u64,
                    "rate-limit pre-emptive backoff"
                );
                time_ext::sleep(delay).await;
            }
        }

        // Build the multipart body manually.
        let boundary = format!("BeetBoundary{:016x}", rand::random::<u64>());
        let body_bytes = build_multipart(&boundary, content, filename, &file_content);
        let content_type = format!("multipart/form-data; boundary={}", boundary);

        let mut req = self.build_request(HttpMethod::Post, &url);
        req.insert_header("content-type", content_type);
        let req = req.with_body(body_bytes);

        let resp = req
            .send()
            .await
            .map_err(|e: beet::core::prelude::BevyError| HttpError::Transport(e.to_string()))?;

        let status = resp.status();
        let rl_info = parse_rate_limit_headers(resp.response_parts());

        {
            let mut limiter = self.limiter.lock().await;
            limiter.update(&route_key, &rl_info);
        }

        let resp_bytes = resp
            .bytes()
            .await
            .map_err(|e: beet::core::prelude::BevyError| HttpError::Transport(e.to_string()))?;

        if status.is_ok() {
            serde_json::from_slice(&resp_bytes).map_err(|e| {
                let raw = String::from_utf8_lossy(&resp_bytes);
                HttpError::Serde(format!("{}: {}", e, &raw[..raw.len().min(200)]))
            })
        } else {
            let status_u16 = status_to_u16(status);
            let body_str = String::from_utf8_lossy(&resp_bytes).to_string();
            Err(HttpError::Api {
                status: status_u16,
                body: body_str,
                route: route_key.to_string(),
            })
        }
    }

    /// Fetch messages from a channel. `query` is appended as a query string
    /// (e.g. `limit=100&before=1234`).
    pub async fn get_messages(
        &self,
        channel_id: &str,
        query: &str,
    ) -> Result<Vec<Message>, HttpError> {
        let path = format!("channels/{}/messages?{}", channel_id, query);
        let route_key = format!("GET /channels/{}/messages", channel_id);
        self.request_json(HttpMethod::Get, &path, &route_key, None)
            .await
    }

    // ------------------------------------------------------------------
    // Convenience: Guilds
    // ------------------------------------------------------------------

    /// Get guild info (with approximate member counts).
    pub async fn get_guild(&self, guild_id: &str) -> Result<Guild, HttpError> {
        let path = format!("guilds/{}?with_counts=true", guild_id);
        let route_key = format!("GET /guilds/{}", guild_id);
        self.request_json(HttpMethod::Get, &path, &route_key, None)
            .await
    }

    // ------------------------------------------------------------------
    // Convenience: Interactions
    // ------------------------------------------------------------------

    /// Respond to an interaction (initial response).
    pub async fn create_interaction_response(
        &self,
        interaction_id: &str,
        interaction_token: &str,
        response: &InteractionResponse,
    ) -> Result<(), HttpError> {
        let path = format!(
            "interactions/{}/{}/callback",
            interaction_id, interaction_token
        );
        let route_key = "POST /interactions/callback".to_string();
        let body = serde_json::to_value(response).map_err(|e| HttpError::Serde(e.to_string()))?;
        // Discord returns 204 No Content on success — don't parse JSON.
        self.request(HttpMethod::Post, &path, &route_key, Some(&body))
            .await?;
        Ok(())
    }

    /// Edit the original interaction response (deferred or follow-up).
    #[allow(dead_code)]
    pub async fn edit_original_interaction_response(
        &self,
        application_id: &str,
        interaction_token: &str,
        body: &serde_json::Value,
    ) -> Result<Message, HttpError> {
        let path = format!(
            "webhooks/{}/{}/messages/@original",
            application_id, interaction_token
        );
        let route_key = "PATCH /webhooks/interaction/messages/@original".to_string();
        self.request_json(HttpMethod::Patch, &path, &route_key, Some(body))
            .await
    }

    // ------------------------------------------------------------------
    // Convenience: Slash command registration
    // ------------------------------------------------------------------

    /// Register (or overwrite) guild-scoped application commands.
    #[allow(unused)]
    pub async fn bulk_overwrite_guild_commands(
        &self,
        application_id: &str,
        guild_id: &str,
        commands: &[ApplicationCommand],
    ) -> Result<Vec<ApplicationCommand>, HttpError> {
        let path = format!(
            "applications/{}/guilds/{}/commands",
            application_id, guild_id
        );
        let route_key = format!(
            "PUT /applications/{}/guilds/{}/commands",
            application_id, guild_id
        );
        let body = serde_json::to_value(commands).map_err(|e| HttpError::Serde(e.to_string()))?;
        self.request_json(HttpMethod::Put, &path, &route_key, Some(&body))
            .await
    }

    /// Register (or overwrite) global application commands.
    pub async fn bulk_overwrite_global_commands(
        &self,
        application_id: &str,
        commands: &[ApplicationCommand],
    ) -> Result<Vec<ApplicationCommand>, HttpError> {
        let path = format!("applications/{}/commands", application_id);
        let route_key = format!("PUT /applications/{}/commands", application_id);
        let body = serde_json::to_value(commands).map_err(|e| HttpError::Serde(e.to_string()))?;
        self.request_json(HttpMethod::Put, &path, &route_key, Some(&body))
            .await
    }

    // ------------------------------------------------------------------
    // Convenience: Channel info
    // ------------------------------------------------------------------

    /// Get channel information.
    #[allow(dead_code)]
    pub async fn get_channel(&self, channel_id: &str) -> Result<Channel, HttpError> {
        let path = format!("channels/{}", channel_id);
        let route_key = format!("GET /channels/{}", channel_id);
        self.request_json(HttpMethod::Get, &path, &route_key, None)
            .await
    }

    // ------------------------------------------------------------------
    // Higher-level helpers
    // ------------------------------------------------------------------

    /// Count messages in a channel by paginating backwards. Caps at 10 000.
    pub async fn count_messages(&self, channel_id: &str) -> Result<usize, HttpError> {
        let mut count = 0usize;
        let mut before: Option<String> = None;
        let max_pages = 100;

        for _ in 0..max_pages {
            let query = match &before {
                Some(b) => format!("limit=100&before={}", b),
                None => "limit=100".to_string(),
            };

            let messages: Vec<Message> = self.get_messages(channel_id, &query).await?;

            if messages.is_empty() {
                break;
            }

            count += messages.len();
            before = messages.last().map(|m| m.id.as_str().to_string());

            if messages.len() < 100 {
                break;
            }
        }

        Ok(count)
    }

    /// Get the very first message ever sent in a channel.
    pub async fn get_first_message(&self, channel_id: &str) -> Result<Message, HttpError> {
        let messages: Vec<Message> = self.get_messages(channel_id, "after=0&limit=1").await?;

        messages.into_iter().next().ok_or_else(|| HttpError::Api {
            status: 404,
            body: "No messages found in this channel.".to_string(),
            route: format!("GET /channels/{}/messages", channel_id),
        })
    }
}

impl std::fmt::Debug for DiscordHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordHttpClient")
            .field("token", &"<redacted>")
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a beet `StatusCode` to a raw HTTP status `u16`.
fn status_to_u16(status: StatusCode) -> u16 {
    match status {
        StatusCode::Ok => 200,
        StatusCode::Created => 201,
        StatusCode::MovedPermanently => 301,
        StatusCode::TemporaryRedirect => 307,
        StatusCode::MalformedRequest => 400,
        StatusCode::Unauthorized => 401,
        StatusCode::Forbidden => 403,
        StatusCode::NotFound => 404,
        StatusCode::MethodNotAllowed => 405,
        StatusCode::RequestTimeout => 408,
        StatusCode::Conflict => 409,
        StatusCode::PayloadTooLarge => 413,
        StatusCode::RateLimitExceeded => 429,
        StatusCode::InternalError => 500,
        StatusCode::NotImplemented => 501,
        StatusCode::ServiceUnavailable => 503,
        StatusCode::GatewayTimeout => 504,
        StatusCode::Http(s) => s.as_u16(),
        _ => 0,
    }
}

/// Build a multipart/form-data body as raw bytes.
///
/// Produces parts for an optional `payload_json` text field and a
/// required file part named `"file"`.
fn build_multipart(
    boundary: &str,
    content: Option<&str>,
    filename: &str,
    file_data: &[u8],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    // Optional payload_json field.
    if let Some(text) = content {
        let payload = json!({ "content": text }).to_string();
        buf.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        buf.extend_from_slice(b"Content-Disposition: form-data; name=\"payload_json\"\r\n");
        buf.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
        buf.extend_from_slice(payload.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    // File part.
    buf.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    buf.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; \
             filename=\"{}\"\r\n",
            filename
        )
        .as_bytes(),
    );
    buf.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    buf.extend_from_slice(file_data);
    buf.extend_from_slice(b"\r\n");

    // Closing boundary.
    buf.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    buf
}
