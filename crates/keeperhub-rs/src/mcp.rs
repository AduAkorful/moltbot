//! Model Context Protocol (MCP) client for KeeperHub.
//!
//! The KeeperHub MCP server is a JSON-RPC 2.0 endpoint over HTTP that
//! exposes 31 tools (docs claim 19 — outdated as of v1.2.0). The full
//! reference is at <https://docs.keeperhub.com/ai-tools/mcp-server>.
//!
//! # Tools (abridged — see `keeperhub-docs-summary.md` for full list)
//!
//! - **Workflow management:** `list_workflows`, `get_workflow`, `create_workflow`,
//!   `update_workflow`, `delete_workflow`, `validate_workflow`, `prepare_test_pin_data`
//! - **Execution:** `execute_workflow`, `get_execution`
//! - **AI generation:** `ai_generate_workflow`
//! - **Discovery:** `list_action_schemas`, `get_plugin`, `search_templates`, `deploy_template`
//! - **Direct DeFi:** `search_protocol_actions`, `execute_protocol_action`,
//!   `execute_transfer`, `execute_contract_call`, `execute_check_and_execute`,
//!   `get_direct_execution_status`
//! - **Marketplace (x402):** `search_workflows`, `call_workflow`, `list_workflow`,
//!   `unlist_workflow`, `update_workflow_listing`, `get_workflow_listing`
//! - **Integrations:** `list_integrations`, `get_wallet_integration`
//! - **Documentation:** `tools_documentation`
//!
//! # Per-workflow servers
//!
//! Every listed marketplace workflow is also reachable as its own narrow
//! MCP server at `/mcp/w/<slug>`. The aggregate server exposes the
//! generic `call_workflow(slug, inputs)` dispatcher; the per-workflow
//! form exposes a single typed tool. This module targets the aggregate
//! server for now; per-workflow servers can be added later.

use crate::error::{Error, Result};
use crate::types::Workflow;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The default KeeperHub MCP endpoint (remote, OAuth).
pub const DEFAULT_MCP_URL: &str = "https://app.keeperhub.com/mcp";

/// MCP client for the KeeperHub server.
///
/// Cheap to clone (wraps an [`Arc`] internally).
#[derive(Debug, Clone)]
pub struct McpClient {
    inner: Arc<McpClientInner>,
}

#[derive(Debug)]
struct McpClientInner {
    /// Base URL of the MCP server.
    url: String,
    /// Authorization header value (e.g. `Bearer kh_...`).
    auth_header: String,
    /// Reusable HTTP client.
    http: reqwest::Client,
}

impl McpClient {
    /// Create a new MCP client with a Bearer API key.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use keeperhub_rs::mcp::McpClient;
    /// let client = McpClient::new("https://app.keeperhub.com/mcp", "kh_your_key");
    /// ```
    pub fn new(url: impl Into<String>, api_key: impl AsRef<str>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client builder should not fail with default config");
        Self {
            inner: Arc::new(McpClientInner {
                url: url.into(),
                auth_header: format!("Bearer {}", api_key.as_ref()),
                http,
            }),
        }
    }

    /// The base URL the client is configured to talk to.
    pub fn url(&self) -> &str {
        &self.inner.url
    }

    /// List all workflows in the authenticated organization.
    ///
    /// Maps to the `list_workflows` MCP tool. Returns an empty vec if the
    /// organization has no workflows yet.
    ///
    /// **Status:** not yet implemented. The function will be wired up in
    /// the next phase of the project plan.
    pub async fn list_workflows(&self) -> Result<Vec<Workflow>> {
        // TODO: POST to {url} with a JSON-RPC `tools/call` envelope
        // { "name": "list_workflows", "arguments": {} }
        // Parse the `content[0].text` JSON into Vec<Workflow>.
        let _ = self; // suppress unused warnings
        Err(Error::Internal(
            "list_workflows not yet implemented — see plans/setup-verified.md"
                .to_string(),
        ))
    }
}

/// A JSON-RPC 2.0 request envelope (used internally for MCP calls).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct JsonRpcRequest {
    pub(crate) jsonrpc: &'static str,
    pub(crate) id: u64,
    pub(crate) method: &'static str,
    pub(crate) params: serde_json::Value,
}

/// A JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct JsonRpcResponse<T> {
    pub(crate) jsonrpc: String,
    pub(crate) id: u64,
    #[serde(default)]
    pub(crate) result: Option<T>,
    #[serde(default)]
    pub(crate) error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct JsonRpcError {
    pub(crate) code: i32,
    pub(crate) message: String,
}
