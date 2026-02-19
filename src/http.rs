//! Transport-abstracted HTTP client for the Discord REST API.
//!
//! All outbound HTTP calls go through [`DiscordHttpClient`] so that auth
//! headers, rate-limit back-off, and error handling live in one place.
//! The underlying `reqwest::Client` is an implementation detail — when we
//! later swap transports we only need to touch this module.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

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
                // Fallback: 1 second from now.
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
// Parse rate-limit headers from a response
// ---------------------------------------------------------------------------

fn parse_rate_limit_headers(headers: &reqwest::header::HeaderMap) -> RateLimitInfo {
    let remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u32>().ok());

    let reset_at = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let reset_after = headers
        .get("x-ratelimit-reset-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let bucket = headers
        .get("x-ratelimit-bucket")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let is_global = headers
        .get("x-ratelimit-global")
        .and_then(|v| v.to_str().ok())
        .map(|s| s == "true")
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
// HTTP method enum (so callers don't pull in reqwest types)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
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
            } => write!(f, "Discord API error {} on {}: {}", status, route, body),
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
    client: reqwest::Client,
    limiter: Arc<Mutex<RateLimiter>>,
}

impl DiscordHttpClient {
    /// Create a new client with the given bot token.
    pub fn new(token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            token: token.into(),
            client,
            limiter: Arc::new(Mutex::new(RateLimiter::new())),
        }
    }

    // ------------------------------------------------------------------
    // Low-level: the single request method everything funnels through
    // ------------------------------------------------------------------

    /// Send a request to `{BASE_URL}/{path}`.
    ///
    /// `route_key` is used for per-route rate-limit bucketing. It should be a
    /// template like `POST /channels/{channel_id}/messages` (major params
    /// filled in, minor params left out).
    ///
    /// Returns the raw response body as bytes on success.
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        route_key: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Vec<u8>, HttpError> {
        // Retry loop for rate-limit 429s.
        let max_retries = 5;
        for attempt in 0..=max_retries {
            // Pre-request: wait if the rate limiter says so.
            {
                let limiter = self.limiter.lock().await;
                if let Some(delay) = limiter.delay_for(route_key) {
                    let delay = delay.min(Duration::from_secs(60));
                    drop(limiter); // release lock while sleeping
                    debug!(
                        route = route_key,
                        delay_ms = delay.as_millis() as u64,
                        "rate-limit pre-emptive backoff"
                    );
                    tokio::time::sleep(delay).await;
                }
            }

            let url = format!("{}/{}", BASE_URL, path.trim_start_matches('/'));

            let reqwest_method = match method {
                Method::Get => reqwest::Method::GET,
                Method::Post => reqwest::Method::POST,
                Method::Put => reqwest::Method::PUT,
                Method::Patch => reqwest::Method::PATCH,
                Method::Delete => reqwest::Method::DELETE,
            };

            let mut builder = self
                .client
                .request(reqwest_method, &url)
                .header("Authorization", format!("Bot {}", self.token));

            if let Some(json) = body {
                builder = builder
                    .header("Content-Type", "application/json")
                    .json(json);
            }

            let resp = builder
                .send()
                .await
                .map_err(|e| HttpError::Transport(e.to_string()))?;

            // Parse rate-limit headers *before* consuming the body.
            let rl_info = parse_rate_limit_headers(resp.headers());
            let status = resp.status();

            // Update the limiter regardless of status.
            {
                let mut limiter = self.limiter.lock().await;
                limiter.update(route_key, &rl_info);
            }

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Discord tells us how long to wait.
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
                    tokio::time::sleep(delay).await;
                    continue;
                }
                // Fall through to the error path below.
            }

            let resp_bytes = resp
                .bytes()
                .await
                .map_err(|e| HttpError::Transport(e.to_string()))?;

            if status.is_success() {
                return Ok(resp_bytes.to_vec());
            }

            let body_str = String::from_utf8_lossy(&resp_bytes).to_string();
            return Err(HttpError::Api {
                status: status.as_u16(),
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
        method: Method,
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
        self.request_json(Method::Post, &path, &route_key, Some(&body))
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

        // Build multipart form
        let mut form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(file_content).file_name(filename.to_string()),
        );

        // Add payload_json if content is provided
        if let Some(text) = content {
            let payload = json!({
                "content": text
            });
            form = form.text("payload_json", payload.to_string());
        }

        // Wait for rate limit
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
                tokio::time::sleep(delay).await;
            }
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| HttpError::Transport(e.to_string()))?;

        // Parse rate-limit headers
        let rl_info = parse_rate_limit_headers(resp.headers());
        let status = resp.status();

        // Update the limiter
        {
            let mut limiter = self.limiter.lock().await;
            limiter.update(&route_key, &rl_info);
        }

        let resp_bytes = resp
            .bytes()
            .await
            .map_err(|e| HttpError::Transport(e.to_string()))?;

        if status.is_success() {
            serde_json::from_slice(&resp_bytes).map_err(|e| {
                let raw = String::from_utf8_lossy(&resp_bytes);
                HttpError::Serde(format!("{}: {}", e, &raw[..raw.len().min(200)]))
            })
        } else {
            let body_str = String::from_utf8_lossy(&resp_bytes).to_string();
            Err(HttpError::Api {
                status: status.as_u16(),
                body: body_str,
                route: route_key.to_string(),
            })
        }
    }

    /// Fetch messages from a channel. `params` are appended as query string
    /// (e.g. `limit=100&before=1234`).
    pub async fn get_messages(
        &self,
        channel_id: &str,
        query: &str,
    ) -> Result<Vec<Message>, HttpError> {
        let path = format!("channels/{}/messages?{}", channel_id, query);
        let route_key = format!("GET /channels/{}/messages", channel_id);
        self.request_json(Method::Get, &path, &route_key, None)
            .await
    }

    // ------------------------------------------------------------------
    // Convenience: Guilds
    // ------------------------------------------------------------------

    /// Get guild info (with approximate member counts).
    pub async fn get_guild(&self, guild_id: &str) -> Result<Guild, HttpError> {
        let path = format!("guilds/{}?with_counts=true", guild_id);
        let route_key = format!("GET /guilds/{}", guild_id);
        self.request_json(Method::Get, &path, &route_key, None)
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
        // Discord returns 204 No Content on success, so we don't parse JSON.
        self.request(Method::Post, &path, &route_key, Some(&body))
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
        self.request_json(Method::Patch, &path, &route_key, Some(body))
            .await
    }

    // ------------------------------------------------------------------
    // Convenience: Slash command registration
    // ------------------------------------------------------------------

    /// Register (or overwrite) guild-scoped application commands.
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
        self.request_json(Method::Put, &path, &route_key, Some(&body))
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
        self.request_json(Method::Put, &path, &route_key, Some(&body))
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
        self.request_json(Method::Get, &path, &route_key, None)
            .await
    }

    // ------------------------------------------------------------------
    // Higher-level helpers (ported from old main.rs)
    // ------------------------------------------------------------------

    /// Count messages in a channel by paginating backwards. Caps at 10 000.
    pub async fn count_messages(&self, channel_id: &str) -> Result<usize, HttpError> {
        let mut count = 0usize;
        let mut before: Option<String> = None;
        let max_pages = 100; // 100 × 100 = 10 000

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
            .field("token", &"[redacted]")
            .finish()
    }
}
