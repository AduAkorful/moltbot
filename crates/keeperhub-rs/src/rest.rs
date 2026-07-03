//! REST API client for KeeperHub.
//!
//! KeeperHub exposes a REST API at `https://app.keeperhub.com/api/v1` for
//! programmatic workflow management, execution, and analytics. This
//! module wraps that surface in an idiomatic Rust client.
//!
//! # Endpoints (planned)
//!
//! - `GET /workflows` — list workflows
//! - `POST /workflows` — create workflow
//! - `GET /workflows/{id}` — get workflow config
//! - `PATCH /workflows/{id}` — update workflow
//! - `DELETE /workflows/{id}` — delete workflow
//! - `POST /workflows/{id}/execute` — trigger execution
//! - `GET /executions/{id}` — execution status
//! - `GET /executions/{id}/logs` — execution logs
//! - `GET /analytics` — usage stats
//! - `GET /chains` — supported chains
//! - `POST /direct-execution` — execute without saving
//!
//! # Auth
//!
//! All endpoints require an org-scoped API key (`kh_` prefix) passed as
//! `Authorization: Bearer <key>`.

use crate::error::{Error, Result};

/// The default KeeperHub REST API base URL.
pub const DEFAULT_REST_URL: &str = "https://app.keeperhub.com/api/v1";

/// REST client for the KeeperHub API.
///
/// Cheap to clone (wraps an [`Arc`] internally).
#[derive(Debug, Clone)]
pub struct RestClient {
    // Inner state will be added when the client is implemented.
    _placeholder: (),
}

impl RestClient {
    /// Create a new REST client with the default base URL.
    pub fn new(_api_key: impl AsRef<str>) -> Self {
        // TODO: build the reqwest client and store the auth header.
        Self { _placeholder: () }
    }

    /// Create a new REST client with a custom base URL (for testing).
    pub fn with_url(_url: impl Into<String>, _api_key: impl AsRef<str>) -> Self {
        Self { _placeholder: () }
    }

    /// Verify the client's credentials and connectivity.
    ///
    /// **Status:** not yet implemented.
    pub async fn ping(&self) -> Result<()> {
        Err(Error::Internal(
            "RestClient::ping not yet implemented".to_string(),
        ))
    }
}
