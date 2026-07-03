//! Telegram alerts.
//!
//! Sends a Telegram message on every safe-mode `Enter` / `Exit`
//! transition (and on any other state change the operator
//! wants to know about). The integration is direct — we
//! POST to Telegram's Bot API rather than going through
//! KeeperHub's Telegram plugin. The direct path is:
//!
//! - **Free** (no x402 cost — the alert path shouldn't pay
//!   for itself)
//! - **Reliable** (Telegram's API is solid; not dependent
//!   on KeeperHub being up)
//! - **Standard** (the same pattern every other Rust
//!   Telegram bot uses)
//!
//! # Setup
//!
//! 1. Talk to [@BotFather](https://t.me/BotFather) on
//!    Telegram, send `/newbot`, follow the prompts, copy
//!    the **bot token** (looks like `123456:ABC-DEF...`).
//! 2. Start a chat with the bot (send any message to it).
//! 3. Visit `https://api.telegram.org/bot<token>/getUpdates`
//!    to find your numeric `chat_id`.
//! 4. Put both in `moltbot.toml` (or set `TELEGRAM_BOT_TOKEN`
//!    as an env var; the chat id has no secret material so
//!    it stays in TOML).
//!
//! # When a message is sent
//!
//! Currently: on safe-mode `Enter` and `Exit` only. A
//! `Telegram::is_configured()` check at the loop boundary
//! short-circuits if either field is missing, so the
//! default `moltbot.toml` (no Telegram config) is a no-op.
//!
//! # Failure mode
//!
//! Telegram API errors (network, 4xx, 5xx) are **logged**
//! but **do not panic, do not return an error to the
//! agent loop**. The alert path is best-effort: missing
//! the operator's attention is bad, but stopping the agent
//! because Telegram is down is worse.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Telegram API errors. `Http` covers reqwest failures;
/// `Api` covers the JSON body returned by Telegram with
/// `ok: false`.
#[derive(Debug, Error)]
pub enum TelegramError {
    #[error("telegram HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("telegram API error: {0}")]
    Api(String),
}

/// A Telegram bot client bound to a specific chat.
///
/// Cheap to clone (the inner `reqwest::Client` is an `Arc`).
#[derive(Debug, Clone)]
pub struct Telegram {
    bot_token: String,
    chat_id: String,
    client: reqwest::Client,
    /// Base URL override for tests. `None` → use the
    /// public `https://api.telegram.org`.
    base_url: Option<String>,
}

impl Telegram {
    /// Build a new Telegram client. The bot token and chat
    /// id must be non-empty; an empty token is treated as
    /// "not configured" by [`Telegram::is_configured`].
    pub fn new(bot_token: impl Into<String>, chat_id: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client builder is infallible");
        Self {
            bot_token: bot_token.into(),
            chat_id: chat_id.into(),
            client,
            base_url: None,
        }
    }

    /// Test-only constructor that points at a mock base URL.
    #[doc(hidden)]
    pub fn with_base_url(
        bot_token: impl Into<String>,
        chat_id: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let mut s = Self::new(bot_token, chat_id);
        s.base_url = Some(base_url.into());
        s
    }

    /// Is this client configured to send? Both `bot_token`
    /// and `chat_id` must be non-empty.
    pub fn is_configured(&self) -> bool {
        !self.bot_token.is_empty() && !self.chat_id.is_empty()
    }

    /// The bot token (used to construct the API URL).
    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }

    /// The chat id the client sends to.
    pub fn chat_id(&self) -> &str {
        &self.chat_id
    }

    /// Build the API URL. Exposed for testing.
    pub fn api_url(&self, method: &str) -> String {
        let base = self.base_url.as_deref().unwrap_or("https://api.telegram.org");
        format!("{base}/bot{}/{method}", self.bot_token)
    }

    /// Send a plain-text message to the bound chat.
    ///
    /// On any error, returns `Err` and **does not** retry.
    /// The caller (the agent loop) logs the error and
    /// continues — see the module-level docs.
    pub async fn send_message(&self, text: &str) -> Result<(), TelegramError> {
        if !self.is_configured() {
            // No-op: a half-configured client (token but no
            // chat, or vice versa) is treated as disabled.
            return Ok(());
        }
        let body = SendMessageRequest {
            chat_id: &self.chat_id,
            text,
        };
        let url = self.api_url("sendMessage");
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let body: TelegramResponse = resp.json().await?;
        if !body.ok {
            return Err(TelegramError::Api(body.description.unwrap_or_else(|| {
                format!("HTTP {} (no description)", status.as_u16())
            })));
        }
        Ok(())
    }

    // -- Message formatters --------------------------------------------
    //
    // The formatters are public so the tick tests can
    // assert the exact text that would be sent, without
    // needing a mock HTTP server. They're also the place
    // to evolve the message format over time.

    /// Format the message for a safe-mode `Enter` event.
    pub fn format_safe_mode_enter(balance_usd: f64, threshold_usd: f64) -> String {
        format!(
            "⚠️ MoltBot: entered safe mode\nbalance: ${balance_usd:.2}\nthreshold: ${threshold_usd:.2}\npaid actions skipped until balance recovers"
        )
    }

    /// Format the message for a safe-mode `Exit` event.
    pub fn format_safe_mode_exit(balance_usd: f64, threshold_usd: f64) -> String {
        format!(
            "✅ MoltBot: exited safe mode\nbalance: ${balance_usd:.2}\nthreshold: ${threshold_usd:.2}\nresuming paid actions"
        )
    }
}

/// The body of a `sendMessage` request. Telegram accepts
/// `chat_id` as either a number or a `@channel` name;
/// we use a `&str` to keep the API flexible.
#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
}

/// Telegram's response envelope. `ok: true` on success;
/// `description` is the human-readable error on failure.
#[derive(Debug, Deserialize)]
struct TelegramResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
}

impl fmt::Display for Telegram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never log the bot token — even a redacted
        // version is risky (Telegram tokens are
        // bearer-equivalent).
        write!(
            f,
            "Telegram(chat_id={}, configured={})",
            self.chat_id,
            self.is_configured()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- is_configured / construction ---------------------------------

    #[test]
    fn is_configured_when_both_set() {
        let t = Telegram::new("123:abc", "987");
        assert!(t.is_configured());
    }

    #[test]
    fn not_configured_with_empty_token() {
        let t = Telegram::new("", "987");
        assert!(!t.is_configured());
    }

    #[test]
    fn not_configured_with_empty_chat_id() {
        let t = Telegram::new("123:abc", "");
        assert!(!t.is_configured());
    }

    #[test]
    fn api_url_uses_public_endpoint_by_default() {
        let t = Telegram::new("123:abc", "987");
        assert_eq!(
            t.api_url("sendMessage"),
            "https://api.telegram.org/bot123:abc/sendMessage"
        );
    }

    #[test]
    fn api_url_respects_base_url_override() {
        let t = Telegram::with_base_url("123:abc", "987", "http://localhost:9999");
        assert_eq!(
            t.api_url("sendMessage"),
            "http://localhost:9999/bot123:abc/sendMessage"
        );
    }

    #[test]
    fn display_does_not_leak_token() {
        let t = Telegram::new("supersecrettoken:abc", "987");
        let s = t.to_string();
        assert!(!s.contains("supersecrettoken"), "display leaked token: {s}");
        assert!(s.contains("987"));
    }

    // -- formatters ---------------------------------------------------

    #[test]
    fn format_safe_mode_enter_contains_balance_and_threshold() {
        let s = Telegram::format_safe_mode_enter(1.23, 5.0);
        assert!(s.contains("entered safe mode"));
        assert!(s.contains("$1.23"));
        assert!(s.contains("$5.00"));
        assert!(s.contains("⚠️") || s.contains("warning") || s.contains("WARN"));
    }

    #[test]
    fn format_safe_mode_exit_contains_balance_and_threshold() {
        let s = Telegram::format_safe_mode_exit(10.5, 5.0);
        assert!(s.contains("exited safe mode"));
        assert!(s.contains("$10.50"));
        assert!(s.contains("$5.00"));
        assert!(s.contains("✅") || s.contains("ok") || s.contains("resuming"));
    }

    #[test]
    fn formatters_handle_zero_and_negative_balance() {
        // Defensive: the function shouldn't crash on edge
        // values. Negative balances are unusual but the
        // formatter just formats the number.
        let s = Telegram::format_safe_mode_enter(-1.0, 5.0);
        assert!(s.contains("$-1.00"));
        let s = Telegram::format_safe_mode_exit(0.0, 5.0);
        assert!(s.contains("$0.00"));
    }

    // -- send_message ------------------------------------------------

    #[tokio::test]
    async fn send_message_is_noop_when_not_configured() {
        // No token, no chat → silent no-op (no HTTP call).
        let t = Telegram::new("", "");
        t.send_message("hello").await.unwrap();
        t.send_message("hello").await.unwrap();
    }
}
