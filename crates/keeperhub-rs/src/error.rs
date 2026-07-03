//! Error types for the KeeperHub Rust client.

use crate::types::PaymentChallenge;
use thiserror::Error;

/// The error type returned by all fallible operations in this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An HTTP transport error (connection, timeout, TLS, etc.).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The KeeperHub API returned an error response.
    #[error("KeeperHub API error (status {status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the server.
        message: String,
    },

    /// The KeeperHub API returned an x402 / MPP 402 challenge (the workflow
    /// is paid and we did not supply payment).
    ///
    /// Callers should inspect the [`PaymentChallenge`] and either:
    /// 1. Use the KeeperHub agentic wallet (`mcp__plugin_keeperhub_wallet__call_workflow`)
    ///    to auto-pay, or
    /// 2. Surface a 402 prompt to the operator.
    #[error("paid workflow '{slug}': {challenge}")]
    X402Unpaid {
        /// The slug of the workflow that required payment.
        slug: String,
        /// The parsed payment challenge. Boxed to keep the `Result` type small.
        challenge: Box<PaymentChallenge>,
    },

    /// Configuration error (missing env var, invalid URL, etc.).
    #[error("configuration error: {0}")]
    Config(String),

    /// A cryptographic operation failed (signing, hashing, etc.).
    #[error("cryptographic error: {0}")]
    Crypto(String),

    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// An MCP-specific error (invalid JSON-RPC, missing tool, etc.).
    #[error("MCP error: {0}")]
    Mcp(String),

    /// A catch-all for unexpected internal errors.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenience alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;
