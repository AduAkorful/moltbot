//! Error types for the KeeperHub Rust client.

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

    /// The KeeperHub API returned an x402 Payment Required challenge that we
    /// could not satisfy (e.g. amount exceeds the auto-approve threshold).
    #[error("x402 payment required but unhandled: {0}")]
    X402Unpaid(String),

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
