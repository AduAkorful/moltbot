//! Model Context Protocol (MCP) client for KeeperHub.
//!
//! The KeeperHub MCP server is a JSON-RPC 2.0 endpoint over HTTP that
//! exposes 31 tools (docs claim 19 — outdated as of v1.2.0) for workflow
//! management, execution, discovery, marketplace calls, and direct DeFi
//! operations. The full reference is at
//! <https://docs.keeperhub.com/ai-tools/mcp-server>.
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
//!
//! # Session model
//!
//! KeeperHub's MCP server uses the Streamable HTTP transport. The handshake
//! is sequential: `initialize` → `notifications/initialized` →
//! `tools/list` (or `tools/call`). The `initialize` response includes an
//! `Mcp-Session-Id` header containing a JWT (24h expiry per the
//! `exp` claim). All subsequent requests must echo this header.
//!
//! [`McpClient`] manages the session lazily: the first call initializes,
//! and the session is reused until it expires or the server returns 401.

use crate::error::{Error, Result};
use crate::types::{CallWorkflowResult, ExecutionDetail, SearchWorkflowsOptions, Workflow};
use crate::x402::parse_challenge;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// The default KeeperHub MCP endpoint (remote, OAuth).
pub const DEFAULT_MCP_URL: &str = "https://app.keeperhub.com/mcp";

/// The MCP protocol version this client speaks.
const PROTOCOL_VERSION: &str = "2025-03-26";

/// A cached MCP session: the JWT returned by `initialize` and its expiry.
#[derive(Debug, Clone)]
struct Session {
    /// The JWT in the `Mcp-Session-Id` header.
    jwt: String,
    /// When this session is no longer trusted to be valid. We set this
    /// to a conservative 23 hours from creation (the server issues 24h
    /// tokens) so we re-initialize before the server rejects us.
    expires_at: std::time::Instant,
}

impl Session {
    fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at
    }
}

/// MCP client for the KeeperHub server.
///
/// Cheap to clone (wraps an [`Arc`] internally). All public methods
/// serialize on a per-client session lock, so concurrent calls from
/// the same client are safe but may block briefly while the session
/// is being initialized or refreshed.
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
    /// Cached session, lazily initialized.
    session: Mutex<Option<Session>>,
    /// Monotonic counter for JSON-RPC request ids.
    next_id: AtomicU64,
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
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client builder should not fail with default config");
        Self {
            inner: Arc::new(McpClientInner {
                url: url.into(),
                auth_header: format!("Bearer {}", api_key.as_ref()),
                http,
                session: Mutex::new(None),
                next_id: AtomicU64::new(1),
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
    /// organization has no workflows yet (which is *not* an error).
    ///
    /// Each workflow includes the full config (nodes, edges, etc.) — the
    /// same shape returned by `get_workflow`. Marketplace listing data
    /// (`listedSlug`, `priceUsdcPerCall`, etc.) is also included if the
    /// workflow is listed.
    pub async fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let text = self.tools_call_text("list_workflows", json!({})).await?;
        if text.is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&text).map_err(|e| {
            Error::Internal(format!(
                "list_workflows: failed to parse response as Vec<Workflow>: {e}; body: {text}"
            ))
        })
    }

    /// Search the KeeperHub marketplace for listed workflows.
    ///
    /// Maps to the `search_workflows` MCP tool. Use this to discover
    /// workflows at runtime (by free-text query, category, chain, or
    /// type) and then call them with [`McpClient::call_workflow`].
    ///
    /// All filters in `opts` are optional. With
    /// [`SearchWorkflowsOptions::default()`] the full marketplace
    /// catalog is returned in the server's default ordering.
    ///
    /// Each returned [`Workflow`] is the slim listing shape (no
    /// `nodes`/`edges` — those come from `get_workflow`).
    ///
    /// Note: the `search_workflows` MCP tool does not expose a `tag`
    /// filter; the available filters are `query`, `category`, `chain`,
    /// `workflowType`, and `sort`. This struct models the real API.
    pub async fn search_workflows(
        &self,
        opts: SearchWorkflowsOptions,
    ) -> Result<Vec<Workflow>> {
        let args = serde_json::to_value(&opts).map_err(|e| {
            Error::Internal(format!("search_workflows: failed to serialize options: {e}"))
        })?;
        let text = self.tools_call_text("search_workflows", args).await?;
        unwrap_search_envelope(&text)
    }

    /// Execute a DeFi protocol action directly, bypassing the visual
    /// workflow builder.
    ///
    /// Maps to the `execute_protocol_action` MCP tool. Use
    /// [`McpClient::search_protocol_actions`] to discover what actions
    /// are available and what params each requires; the `action_type`
    /// follows the `protocol/action-slug` format (e.g. `aave-v3/supply`,
    /// `morpho/get-position`).
    ///
    /// `params` is a free-form JSON object whose shape depends on the
    /// action. The KeeperHub-side protocol-specific params (network,
    /// wallet) and the protocol-side action params (asset, amount, etc.)
    /// are all packed into the same object.
    ///
    /// Note: protocol actions are *plugins* (not credentialed
    /// integrations). Write actions need a wallet — the existing org
    /// `web3` integration is used implicitly when exactly one is
    /// configured.
    pub async fn execute_protocol_action(
        &self,
        action_type: &str,
        params: Value,
    ) -> Result<Value> {
        let text = self
            .tools_call_text(
                "execute_protocol_action",
                json!({
                    "actionType": action_type,
                    "params": params,
                }),
            )
            .await?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| {
            Error::Internal(format!(
                "execute_protocol_action({action_type}): failed to parse response: {e}; body: {text}"
            ))
        })
    }

    /// Search for available DeFi protocol actions.
    ///
    /// Maps to the `search_protocol_actions` MCP tool. Use to discover
    /// supported protocols and their action slugs before calling
    /// [`McpClient::execute_protocol_action`].
    pub async fn search_protocol_actions(
        &self,
        query: Option<&str>,
        protocol: Option<&str>,
    ) -> Result<Value> {
        let mut args = serde_json::Map::new();
        if let Some(q) = query {
            args.insert("query".to_string(), Value::String(q.to_string()));
        }
        if let Some(p) = protocol {
            args.insert("protocol".to_string(), Value::String(p.to_string()));
        }
        let text = self
            .tools_call_text("search_protocol_actions", Value::Object(args))
            .await?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| {
            Error::Internal(format!(
                "search_protocol_actions: failed to parse response: {e}; body: {text}"
            ))
        })
    }

    /// Call a marketplace workflow by slug.
    ///
    /// For **free** workflows, returns the [`CallWorkflowResult`] with
    /// `executionId`, `status`, `output`, and optional ERC-8004
    /// `feedback` prompt.
    ///
    /// For **paid** workflows, returns
    /// [`Error::X402Unpaid`] with the parsed [`PaymentChallenge`].
    /// Callers should then either:
    ///
    /// 1. Invoke the KeeperHub agentic wallet's MCP server
    ///    (`mcp__plugin_keeperhub_wallet__call_workflow`) to auto-pay
    ///    and retry, or
    /// 2. Surface a 402 prompt to the operator.
    ///
    /// The Rust client does **not** reimplement EIP-3009 signing —
    /// see [`crate::x402`] for why.
    pub async fn call_workflow(&self, slug: &str, inputs: Value) -> Result<CallWorkflowResult> {
        match self
            .tools_call_text(
                "call_workflow",
                json!({ "slug": slug, "inputs": inputs }),
            )
            .await
        {
            Ok(text) => serde_json::from_str(&text).map_err(|e| {
                Error::Internal(format!(
                    "call_workflow({slug}): failed to parse response as CallWorkflowResult: {e}; body: {text}"
                ))
            }),
            Err(Error::Api { status: 402, message }) => {
                let challenge = extract_challenge_from_402(&message).ok_or_else(|| {
                    Error::Mcp(format!(
                        "call_workflow({slug}): 402 returned but could not parse x402 challenge: {message}"
                    ))
                })?;
                Err(Error::X402Unpaid {
                    slug: slug.to_string(),
                    challenge: Box::new(challenge),
                })
            }
            Err(other) => Err(other),
        }
    }

    /// Fetch the full execution record + status summary for a previous
    /// workflow execution.
    ///
    /// This is the audit-trail retrieval tool. After calling
    /// [`McpClient::execute_workflow`] (or running a free workflow via
    /// [`McpClient::call_workflow`]), poll this method to get the
    /// per-node statuses, the final output, any onchain transaction
    /// hashes, and the audit trail (trigger source, who triggered it,
    /// the workflow hash at execution time, etc.).
    ///
    /// The returned [`ExecutionDetail`] combines the `status` envelope
    /// and the full `execution` record (the API's `logs` wrapper is
    /// flattened for ergonomics).
    pub async fn get_execution(&self, execution_id: &str) -> Result<ExecutionDetail> {
        let text = self
            .tools_call_text(
                "get_execution",
                json!({ "executionId": execution_id }),
            )
            .await?;
        flatten_logs_envelope(&text)
    }

    /// Manually trigger a workflow's execution. Returns the execution ID
    /// for status polling.
    ///
    /// Note: this is for workflows in your own org, called directly by
    /// ID (not by slug from the marketplace). For marketplace workflows,
    /// use [`McpClient::call_workflow`].
    pub async fn execute_workflow(&self, workflow_id: &str) -> Result<String> {
        let text = self
            .tools_call_text(
                "execute_workflow",
                json!({ "workflowId": workflow_id }),
            )
            .await?;
        let parsed: Value = serde_json::from_str(&text).map_err(|e| {
            Error::Internal(format!(
                "execute_workflow({workflow_id}): failed to parse response: {e}; body: {text}"
            ))
        })?;
        parsed
            .get("executionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                Error::Mcp(format!(
                    "execute_workflow({workflow_id}): response missing executionId; body: {text}"
                ))
            })
    }

    /// Initialize the MCP session (handshake).
    ///
    /// Sends `initialize`, captures the `Mcp-Session-Id` JWT from the
    /// response header, then sends `notifications/initialized`. This is
    /// called automatically by [`McpClient::list_workflows`] (and every
    /// other public method), so you shouldn't need to call it directly.
    /// It's exposed publicly for callers that want to fail fast at
    /// startup rather than on first call.
    pub async fn initialize(&self) -> Result<()> {
        self.ensure_session().await?;
        Ok(())
    }

    /// Internal: ensure we have a valid session, creating one if needed.
    async fn ensure_session(&self) -> Result<Session> {
        {
            let guard = self.inner.session.lock().await;
            if let Some(s) = guard.as_ref() {
                if !s.is_expired() {
                    return Ok(s.clone());
                }
            }
        }
        let session = self.initialize_session().await?;
        let mut guard = self.inner.session.lock().await;
        *guard = Some(session.clone());
        Ok(session)
    }

    async fn initialize_session(&self) -> Result<Session> {
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "keeperhub-rs",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        });

        let resp = self
            .inner
            .http
            .post(&self.inner.url)
            .header(reqwest::header::AUTHORIZATION, &self.inner.auth_header)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&init_request)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        // The JWT is in the Mcp-Session-Id response header. We don't
        // need the body for the handshake, but we consume it to avoid
        // a connection-leak warning.
        let jwt = resp
            .headers()
            .get("Mcp-Session-Id")
            .ok_or_else(|| {
                Error::Mcp("initialize response missing Mcp-Session-Id header".to_string())
            })?
            .to_str()
            .map_err(|e| Error::Mcp(format!("Mcp-Session-Id is not valid UTF-8: {e}")))?
            .to_string();
        let _ = resp.text().await?;

        // Send notifications/initialized with the session header.
        let note = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let _note_resp = self
            .inner
            .http
            .post(&self.inner.url)
            .header(reqwest::header::AUTHORIZATION, &self.inner.auth_header)
            .header("Mcp-Session-Id", &jwt)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&note)
            .send()
            .await?;

        // The server returns HTTP 202 for notifications. We don't check
        // the status — even on error the session is still usable.
        let _ = _note_resp.text().await?;

        // Set expiry to 23h from now. The server issues 24h tokens; we
        // refresh early to avoid races at the boundary.
        Ok(Session {
            jwt,
            expires_at: std::time::Instant::now() + Duration::from_secs(23 * 3600),
        })
    }

    /// Internal: send a JSON-RPC request, returning the `result` field
    /// (or an error if the response is `error` or HTTP non-2xx).
    async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let session = self.ensure_session().await?;

        let id = self.next_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let resp = self
            .inner
            .http
            .post(&self.inner.url)
            .header(reqwest::header::AUTHORIZATION, &self.inner.auth_header)
            .header("Mcp-Session-Id", &session.jwt)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&request)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let parsed: JsonRpcResponse<Value> = serde_json::from_str(&body).map_err(|e| {
            Error::Mcp(format!(
                "failed to parse JSON-RPC response for {method}: {e}; body: {body}"
            ))
        })?;

        if let Some(err) = parsed.error {
            return Err(Error::Mcp(format!("{}: {}", err.code, err.message)));
        }

        parsed.result.ok_or_else(|| {
            Error::Mcp(format!(
                "JSON-RPC response for {method} had neither result nor error; body: {body}"
            ))
        })
    }

    /// Internal: call an MCP tool and return the unwrapped text content.
    ///
    /// `tools/call` responses wrap the data in a `content: [{type, text}]`
    /// envelope. This helper unwraps it, or returns `Error::Api` if the
    /// response is marked `isError: true`.
    async fn tools_call_text(&self, name: &str, arguments: Value) -> Result<String> {
        let result = self
            .send_request("tools/call", json!({ "name": name, "arguments": arguments }))
            .await?;
        unwrap_content(&result)
    }

    fn next_id(&self) -> u64 {
        self.inner.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// A JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse<T> {
    #[serde(default)]
    result: Option<T>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Unwrap a `tools/call` result envelope.
///
/// On success, returns the first text content. On `isError: true`, parses
/// the text for the underlying status code and message and returns
/// `Error::Api`.
fn unwrap_content(result: &Value) -> Result<String> {
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::Mcp("tools/call response missing content array".to_string()))?;

    let first = content
        .first()
        .ok_or_else(|| Error::Mcp("tools/call response has empty content array".to_string()))?;

    let text = first
        .get("text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::Mcp("tools/call content block is not text".to_string()))?
        .to_string();

    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_error {
        let (status, message) = parse_api_error_text(&text);
        return Err(Error::Api { status, message });
    }

    Ok(text)
}

/// Parse an error text from KeeperHub into (status, message).
///
/// KeeperHub returns errors in the form
/// `"API call failed: 503 Service Unavailable - <body>"`. We extract the
/// status code if present; otherwise default to 500 with the full text.
fn parse_api_error_text(text: &str) -> (u16, String) {
    let Some(rest) = text.strip_prefix("API call failed: ") else {
        return (500, text.to_string());
    };

    // Find the first space (between status code and reason phrase).
    let Some(space_idx) = rest.find(' ') else {
        // Maybe the text is just "API call failed: 503"
        if let Ok(status) = rest.trim().parse::<u16>() {
            return (status, text.to_string());
        }
        return (500, text.to_string());
    };

    if let Ok(status) = rest[..space_idx].parse::<u16>() {
        return (status, text.to_string());
    }

    (500, text.to_string())
}

/// Extract an x402 challenge from a 402 error message.
///
/// Expects the format `"API call failed: 402 Payment Required - {json}"`.
/// Returns `None` if the message doesn't match the expected shape or the
/// JSON can't be parsed as a [`PaymentChallenge`].
fn extract_challenge_from_402(message: &str) -> Option<crate::types::PaymentChallenge> {
    let (status, _) = parse_api_error_text(message);
    if status != 402 {
        return None;
    }
    // Find " - " separator. The challenge JSON comes after.
    let dash_idx = message.find(" - ")?;
    let json_str = message.get(dash_idx + 3..)?;
    parse_challenge(json_str).ok()
}

/// Flatten the `get_execution` response from `{ status, logs: { execution } }`
/// to `{ status, execution }` for ergonomic deserialization into
/// [`ExecutionDetail`].
fn flatten_logs_envelope(text: &str) -> Result<crate::types::ExecutionDetail> {
    let mut v: Value = serde_json::from_str(text).map_err(|e| {
        Error::Internal(format!(
            "get_execution: response is not valid JSON: {e}; body: {text}"
        ))
    })?;
    if let Some(logs) = v.get_mut("logs").and_then(|l| l.as_object_mut()) {
        if let Some(exec) = logs.remove("execution") {
            v.as_object_mut()
                .ok_or_else(|| {
                    Error::Internal(format!(
                        "get_execution: response is not a JSON object; body: {text}"
                    ))
                })?
                .insert("execution".to_string(), exec);
        }
    }
    serde_json::from_value(v).map_err(|e| {
        Error::Internal(format!(
            "get_execution: failed to parse response as ExecutionDetail: {e}; body: {text}"
        ))
    })
}

/// Unwrap the `search_workflows` response from `{ items: [Workflow, ...] }`
/// into `Vec<Workflow>`.
///
/// The MCP server wraps the items array in an envelope object. The
/// envelope may also include pagination fields (e.g. `nextCursor`) in
/// future versions — we ignore anything except `items`.
fn unwrap_search_envelope(text: &str) -> Result<Vec<Workflow>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let v: Value = serde_json::from_str(text).map_err(|e| {
        Error::Internal(format!(
            "search_workflows: response is not valid JSON: {e}; body: {text}"
        ))
    })?;
    let items = v
        .get("items")
        .and_then(|i| i.as_array())
        .ok_or_else(|| {
            Error::Internal(format!(
                "search_workflows: response missing 'items' array; body: {text}"
            ))
        })?;
    serde_json::from_value(Value::Array(items.clone())).map_err(|e| {
        Error::Internal(format!(
            "search_workflows: failed to parse items as Vec<Workflow>: {e}; body: {text}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_content_returns_text_on_success() {
        let result = json!({
            "content": [{"type": "text", "text": "[]"}]
        });
        assert_eq!(unwrap_content(&result).unwrap(), "[]");
    }

    #[test]
    fn unwrap_content_returns_api_error_on_is_error() {
        let result = json!({
            "content": [{"type": "text", "text": "API call failed: 503 Service Unavailable - {\"error\":\"Workflow temporarily unavailable\"}"}],
            "isError": true
        });
        let err = unwrap_content(&result).unwrap_err();
        match err {
            Error::Api { status, message } => {
                assert_eq!(status, 503);
                assert!(message.contains("503"));
            }
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[test]
    fn unwrap_content_errors_on_missing_content() {
        let result = json!({"foo": "bar"});
        let err = unwrap_content(&result).unwrap_err();
        assert!(matches!(err, Error::Mcp(_)));
    }

    #[test]
    fn parse_api_error_text_extracts_status() {
        assert_eq!(
            parse_api_error_text("API call failed: 404 Not Found - {\"error\":\"missing\"}"),
            (404, "API call failed: 404 Not Found - {\"error\":\"missing\"}".to_string())
        );
        assert_eq!(
            parse_api_error_text("API call failed: 422 - bad input"),
            (422, "API call failed: 422 - bad input".to_string())
        );
    }

    #[test]
    fn parse_api_error_text_defaults_to_500() {
        assert_eq!(
            parse_api_error_text("something weird happened"),
            (500, "something weird happened".to_string())
        );
    }

    #[test]
    fn extract_challenge_from_402_parses_well_formed_body() {
        let msg = r#"API call failed: 402 Payment Required - {"protocol":"x402","amount":"10000","asset":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","chainId":8453,"payTo":"0xabc","nonce":"0xn","validAfter":0,"validBefore":9999999999,"resource":"my-workflow"}"#;
        let challenge = extract_challenge_from_402(msg).expect("should parse");
        assert_eq!(challenge.amount, "10000");
        assert_eq!(challenge.chain_id, 8453);
        assert_eq!(challenge.resource.as_deref(), Some("my-workflow"));
    }

    #[test]
    fn extract_challenge_from_402_rejects_non_402() {
        let msg = "API call failed: 503 Service Unavailable - {}";
        assert!(extract_challenge_from_402(msg).is_none());
    }

    #[test]
    fn extract_challenge_from_402_rejects_malformed_json() {
        let msg = "API call failed: 402 Payment Required - not json at all";
        assert!(extract_challenge_from_402(msg).is_none());
    }

    #[test]
    fn flatten_logs_envelope_lifts_execution() {
        let body = r#"{
            "status": { "status": "success" },
            "logs": { "execution": { "id": "abc", "workflowId": "wf1", "status": "success" } }
        }"#;
        let detail = flatten_logs_envelope(body).unwrap();
        assert_eq!(detail.status.status, crate::types::ExecutionStatus::Success);
        assert_eq!(detail.execution.id, "abc");
        assert_eq!(detail.execution.workflow_id, "wf1");
    }

    #[test]
    fn flatten_logs_envelope_handles_missing_logs() {
        let body = r#"{ "status": { "status": "failed" } }"#;
        // The Execution will fail to deserialize (missing required fields)
        // but we should at least handle the parse and surface a clear error.
        let err = flatten_logs_envelope(body).unwrap_err();
        assert!(matches!(err, Error::Internal(_)));
    }

    #[test]
    fn unwrap_search_envelope_returns_empty_for_empty_string() {
        let items = unwrap_search_envelope("").unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn unwrap_search_envelope_returns_empty_vec_for_empty_items() {
        let body = r#"{ "items": [] }"#;
        let items = unwrap_search_envelope(body).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn unwrap_search_envelope_parses_slim_workflow() {
        let body = r#"{
            "items": [{
                "id": "abc123",
                "name": "Aave Health Check",
                "description": "Returns the user's Aave V3 health factor.",
                "listedSlug": "aave-health-check",
                "listedAt": "2026-07-01T12:00:00.000Z",
                "inputSchema": {"type": "object", "properties": {"wallet": {"type": "string"}}},
                "outputMapping": {"result": "{{@node:Result.result}}"},
                "priceUsdcPerCall": "0.05",
                "organizationId": "org-1",
                "createdAt": "2026-07-01T11:00:00.000Z",
                "updatedAt": "2026-07-01T12:00:00.000Z",
                "isListed": true,
                "workflowType": "read",
                "category": "defi",
                "chain": "1"
            }]
        }"#;
        let items = unwrap_search_envelope(body).unwrap();
        assert_eq!(items.len(), 1);
        let w = &items[0];
        assert_eq!(w.id, "abc123");
        assert_eq!(w.name, "Aave Health Check");
        assert_eq!(w.listed_slug.as_deref(), Some("aave-health-check"));
        assert_eq!(w.price_usdc_per_call.as_deref(), Some("0.05"));
        assert_eq!(w.category.as_deref(), Some("defi"));
        assert_eq!(w.chain.as_deref(), Some("1"));
        assert!(w.is_listed);
        // Slim shape: nodes/edges are None
        assert!(w.nodes.is_none());
        assert!(w.edges.is_none());
        assert!(w.input_schema.is_some());
    }

    #[test]
    fn unwrap_search_envelope_errors_on_missing_items() {
        let body = r#"{ "nextCursor": "xyz" }"#;
        let err = unwrap_search_envelope(body).unwrap_err();
        match err {
            Error::Internal(msg) => {
                assert!(msg.contains("missing 'items'"), "unexpected error: {msg}");
            }
            other => panic!("expected Error::Internal, got {other:?}"),
        }
    }

    #[test]
    fn unwrap_search_envelope_errors_on_invalid_json() {
        let body = "this is not json";
        let err = unwrap_search_envelope(body).unwrap_err();
        assert!(matches!(err, Error::Internal(_)));
    }

    #[test]
    fn search_workflows_options_default_serializes_to_empty_object() {
        let opts = SearchWorkflowsOptions::default();
        let json = serde_json::to_value(&opts).unwrap();
        // Every field is None, so the serialized form is `{}`.
        assert_eq!(json, json!({}));
    }

    #[test]
    fn search_workflows_options_skips_none_fields() {
        let opts = SearchWorkflowsOptions {
            query: Some("morpho".into()),
            category: Some("defi".into()),
            chain: None,
            workflow_type: Some("read".into()),
            sort: None,
        };
        let json = serde_json::to_value(&opts).unwrap();
        // Only set fields are present; None fields are omitted.
        assert_eq!(
            json,
            json!({
                "query": "morpho",
                "category": "defi",
                "workflowType": "read",
            })
        );
    }

    #[test]
    fn search_workflows_options_round_trips_via_camelcase() {
        let opts = SearchWorkflowsOptions {
            workflow_type: Some("write".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&opts).unwrap();
        // Verify the wire format uses camelCase.
        assert!(json.contains("workflowType"), "expected camelCase in {json}");
        // And round-trips back to the same struct.
        let back: SearchWorkflowsOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(back.workflow_type.as_deref(), Some("write"));
        assert!(back.query.is_none());
    }
}
