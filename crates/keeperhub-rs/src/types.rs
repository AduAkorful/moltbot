//! Shared types used across the KeeperHub Rust client.
//!
//! These types map closely to the JSON shapes returned by the KeeperHub
//! REST API and MCP server. They are intentionally permissive (most
//! fields are `Option` with `#[serde(default)]`) so that a single type
//! can model both the full workflow object (returned by `list_workflows`,
//! `get_workflow`) and the slimmer listing (returned by `search_workflows`,
//! `get_workflow_listing`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


/// A KeeperHub workflow.
///
/// Workflows are the unit of automation. Each has triggers, actions, and
/// optionally conditions. The struct here models the full object — both
/// the in-org representation and the marketplace listing share this type,
/// with unused fields being `None` in the listing case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    /// Unique workflow ID.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the workflow is enabled (scheduled/event triggers will fire).
    #[serde(default)]
    pub enabled: bool,

    /// Visibility: `"private"`, `"public"`, etc.
    #[serde(default)]
    pub visibility: Option<String>,

    /// Whether the workflow is published to the marketplace.
    #[serde(default)]
    pub is_listed: bool,

    /// Marketplace slug. Set when `is_listed` is true. Slugs are
    /// **permanent once set** — KeeperHub does not let you rename them.
    #[serde(default)]
    pub listed_slug: Option<String>,

    /// When the workflow was listed to the marketplace.
    #[serde(default)]
    pub listed_at: Option<DateTime<Utc>>,

    /// Per-call price in USDC. String per the API (e.g. `"0.01"`).
    /// `None` for free workflows.
    #[serde(default)]
    pub price_usdc_per_call: Option<String>,

    /// Workflow type: `"read"` or `"write"`. Relevant for marketplace listings.
    #[serde(default)]
    pub workflow_type: Option<String>,

    /// Marketplace category (e.g. `"defi"`, `"automation"`).
    #[serde(default)]
    pub category: Option<String>,

    /// Chain the workflow operates on (e.g. `"8453"` for Base, `"1"` for Ethereum).
    #[serde(default)]
    pub chain: Option<String>,

    /// Input JSON schema for callers. Always present on listed workflows.
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,

    /// Output mapping (which node outputs are exposed to callers).
    #[serde(default)]
    pub output_mapping: Option<serde_json::Value>,

    /// Listing schema version.
    #[serde(default)]
    pub listing_version: Option<u32>,

    /// Project ID (KeeperHub org-internal grouping).
    #[serde(default)]
    pub project_id: Option<String>,

    /// Tag ID.
    #[serde(default)]
    pub tag_id: Option<String>,

    /// ID of the user that owns the workflow.
    #[serde(default)]
    pub user_id: Option<String>,

    /// Organization ID.
    #[serde(default)]
    pub organization_id: Option<String>,

    /// Whether the workflow is featured in the marketplace.
    #[serde(default)]
    pub featured: Option<bool>,

    /// Workflow nodes (triggers, actions, conditions).
    #[serde(default)]
    pub nodes: Option<Vec<Node>>,

    /// Workflow edges (node connections).
    #[serde(default)]
    pub edges: Option<Vec<Edge>>,

    /// When the workflow was created.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// When the workflow was last updated.
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,

    /// When the workflow was soft-deleted (if applicable).
    #[serde(default)]
    pub deleted_at: Option<DateTime<Utc>>,

    /// When the workflow was deactivated (if applicable).
    #[serde(default)]
    pub deactivated_at: Option<DateTime<Utc>>,
}

/// A node in a workflow's graph (trigger, action, or condition).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Stable node ID within the workflow. Referenced by edges.
    pub id: String,

    /// Node kind. Usually matches `data.type` (e.g. `"trigger"`, `"action"`).
    #[serde(default, rename = "type")]
    pub node_type: Option<String>,

    /// Position on the visual canvas. May be `None` for headless workflows.
    #[serde(default)]
    pub position: Option<NodePosition>,

    /// Node payload (label, status, type-specific config).
    pub data: NodeData,
}

/// A node's (x, y) position on the visual canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    /// Horizontal position.
    pub x: f64,
    /// Vertical position.
    pub y: f64,
}

/// The data payload of a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeData {
    /// Type discriminator. For triggers, the value is `"trigger"` and
    /// `config.triggerType` distinguishes Manual / Schedule / Webhook / etc.
    /// For actions, the value is `"action"` and `config.actionType` names
    /// the action (e.g. `"web3/check-balance"`, `"aave-v3/supply"`).
    #[serde(default, rename = "type")]
    pub data_type: Option<String>,

    /// Human-readable label (e.g. "Check SepoliaETH balance").
    #[serde(default)]
    pub label: Option<String>,

    /// Type-specific config. The shape varies by node type. For triggers
    /// it includes `triggerType` + (for Schedule) a cron expression. For
    /// actions it includes `actionType` + the required input fields.
    #[serde(default)]
    pub config: serde_json::Value,

    /// Node status (e.g. `"idle"`, `"running"`, `"success"`, `"failed"`).
    #[serde(default)]
    pub status: Option<String>,

    /// Optional description shown in the visual builder.
    #[serde(default)]
    pub description: Option<String>,
}

/// An edge in a workflow's graph, connecting two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Stable edge ID within the workflow.
    pub id: String,

    /// Source node ID.
    pub source: String,

    /// Target node ID.
    pub target: String,

    /// Source output handle. Required for condition nodes (`"true"`/`"false"`)
    /// and for-each nodes (`"loop"`/`"done"`). Omitted for other node types.
    #[serde(default)]
    pub source_handle: Option<String>,
}

/// The status of a single workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// Queued for execution.
    Pending,
    /// Currently running.
    Running,
    /// Completed successfully.
    Completed,
    /// Completed successfully (alias returned by `get_execution`).
    Success,
    /// Failed (check logs for the error).
    Failed,
    /// Cancelled by a user.
    Cancelled,
}

impl ExecutionStatus {
    /// Returns true if the execution is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Success | Self::Failed | Self::Cancelled
        )
    }

    /// Returns `true` if the execution finished without error.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed | Self::Success)
    }
}

/// The full audit-trail response from `get_execution`.
///
/// Returned by [`crate::mcp::McpClient::get_execution`]. Combines a
/// status summary with the full execution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionDetail {
    /// High-level status summary (status, per-node statuses, progress).
    pub status: ExecutionStatusSummary,

    /// The full execution record: input, output, audit trail.
    /// (The API wraps this in a `logs` envelope; the wrapper is
    /// flattened on deserialization for ergonomics.)
    pub execution: Execution,
}

/// The `status` half of the `get_execution` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStatusSummary {
    /// Top-level status.
    pub status: ExecutionStatus,

    /// Per-node status (one per workflow node that ran).
    #[serde(default)]
    pub node_statuses: Vec<NodeStatus>,

    /// Progress metadata.
    #[serde(default)]
    pub progress: Option<ExecutionProgress>,

    /// Transaction hashes produced so far (may include pre-completion
    /// hashes while a multi-node execution is still running).
    #[serde(default)]
    pub transaction_hashes: Vec<ExecutionTxHash>,

    /// Optional error context when the execution failed.
    #[serde(default)]
    pub error_context: Option<serde_json::Value>,
}

/// Status of a single node within an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    /// The node ID within the workflow.
    pub node_id: String,

    /// Node status (`"success"`, `"running"`, `"failed"`, etc.).
    pub status: String,
}

/// Progress metadata for an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionProgress {
    /// Total number of steps in the workflow (if known).
    #[serde(default)]
    pub total_steps: Option<u32>,
    /// Number of steps that have completed.
    #[serde(default)]
    pub completed_steps: u32,
    /// Number of steps currently running.
    #[serde(default)]
    pub running_steps: u32,
    /// The node currently executing.
    #[serde(default)]
    pub current_node_id: Option<String>,
    /// The display name of the node currently executing.
    #[serde(default)]
    pub current_node_name: Option<String>,
    /// Progress as a 0-100 percentage.
    #[serde(default)]
    pub percentage: u32,
}

/// A transaction hash produced by an execution, attributed to a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTxHash {
    /// The transaction hash (0x...).
    pub hash: String,

    /// The node that produced this tx.
    #[serde(default)]
    pub node_id: Option<String>,

    /// Human-readable node name.
    #[serde(default)]
    pub node_name: Option<String>,
}

/// A single execution of a workflow — the full audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    /// Unique execution ID (a base62 string from the API).
    pub id: String,

    /// The workflow that was executed.
    pub workflow_id: String,

    /// Current status.
    pub status: ExecutionStatus,

    /// Inputs to the execution (workflow-trigger inputs).
    #[serde(default)]
    pub input: serde_json::Value,

    /// Output of the execution (workflow final-node output).
    #[serde(default)]
    pub output: serde_json::Value,

    /// Error message if the execution failed.
    #[serde(default)]
    pub error: Option<String>,

    /// Error category (e.g. `"TIMEOUT"`, `"RPC_ERROR"`).
    #[serde(default)]
    pub error_category: Option<String>,

    /// Error type (e.g. `"network"`).
    #[serde(default)]
    pub error_type: Option<String>,

    /// Numeric error code.
    #[serde(default)]
    pub error_code: Option<String>,

    /// When the execution started.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,

    /// When the execution ended (if terminal).
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,

    /// Duration string (e.g. `"400"` for 400ms). String per the API.
    #[serde(default)]
    pub duration: Option<String>,

    /// KeeperHub run ID (different from the MCP execution ID).
    #[serde(default)]
    pub run_id: Option<String>,

    /// Transaction hashes produced by this execution.
    #[serde(default)]
    pub transaction_hashes: Vec<ExecutionTxHash>,

    /// Total gas used across all transactions (in wei).
    #[serde(default)]
    pub gas_used_wei: Option<String>,

    /// Whether this execution counts against the org's monthly quota.
    #[serde(default)]
    pub billable: Option<bool>,

    /// What triggered this execution (`"manual"`, `"schedule"`, etc.).
    #[serde(default)]
    pub trigger_source: Option<String>,

    /// Org API key that triggered the execution (if applicable).
    #[serde(default)]
    pub triggered_by_org_api_key_id: Option<String>,

    /// User API key that triggered the execution (if applicable).
    #[serde(default)]
    pub triggered_by_user_api_key_id: Option<String>,

    /// Type of credential that triggered the execution.
    #[serde(default)]
    pub triggered_by_credential_type: Option<String>,

    /// Ordered list of node IDs that ran.
    #[serde(default)]
    pub execution_trace: Option<Vec<String>>,

    /// Last node that completed successfully.
    #[serde(default)]
    pub last_successful_node_id: Option<String>,

    /// Last node's display name.
    #[serde(default)]
    pub last_successful_node_name: Option<String>,

    /// Hash of the workflow's node graph at execution time.
    #[serde(default)]
    pub executed_workflow_hash: Option<String>,

    /// Full embedded workflow (useful for forensic audit). Stored as
    /// `serde_json::Value` to avoid a recursive type cycle: Workflow
    /// doesn't contain Execution, but Execution can be returned with
    /// the workflow embedded for one-shot audit views.
    #[serde(default)]
    pub embedded_workflow: Option<serde_json::Value>,
}

/// A single log entry in an execution's audit trail.
///
/// Note: the `get_execution` response embeds the full execution record
/// (which includes the audit trail), so this `LogEntry` type is more of
/// a future-proof schema. The current API uses `nodeStatuses` + the
/// `execution` object's fields to surface the per-node log data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// The node that produced this entry.
    pub node_id: String,

    /// The node's display label.
    pub label: String,

    /// The action type (e.g. `web3/transfer-token`).
    pub action_type: String,

    /// The entry's timestamp.
    pub timestamp: DateTime<Utc>,

    /// The node's output (may be a primitive, object, or array).
    pub output: serde_json::Value,

    /// Onchain tx hash if this node produced one.
    #[serde(default)]
    pub tx_hash: Option<String>,

    /// Gas used for the tx (if applicable).
    #[serde(default)]
    pub gas_used: Option<String>,

    /// Error message if the node failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// The payment protocol used by a 402 challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentProtocol {
    /// HTTP 402 + EIP-3009 `TransferWithAuthorization` on Base.
    X402,
    /// MPP on Tempo (Tempo's native payment protocol).
    Mpp,
}

/// A parsed x402 / MPP 402 challenge body, returned in
/// [`crate::error::Error::X402Unpaid`] when a paid workflow is called
/// without supplying payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentChallenge {
    /// The payment protocol.
    pub protocol: PaymentProtocol,

    /// The amount to pay, in atomic units of the asset (e.g. `50000` = $0.05 USDC).
    pub amount: String,

    /// The asset contract address.
    pub asset: String,

    /// The chain ID where the payment should settle.
    pub chain_id: u64,

    /// The facilitator's address that will receive the funds.
    pub pay_to: String,

    /// A unique nonce for this payment.
    pub nonce: String,

    /// Unix timestamp after which the authorization expires.
    pub valid_after: u64,

    /// Unix timestamp before which the authorization is valid.
    pub valid_before: u64,

    /// Optional resource identifier (the workflow being paid for).
    #[serde(default)]
    pub resource: Option<String>,
}

impl std::fmt::Display for PaymentChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} chain={} amount={} asset={} pay_to={}",
            match self.protocol {
                PaymentProtocol::X402 => "x402",
                PaymentProtocol::Mpp => "MPP",
            },
            self.chain_id,
            self.amount,
            self.asset,
            self.pay_to,
        )?;
        if let Some(r) = &self.resource {
            write!(f, " resource={r}")?;
        }
        Ok(())
    }
}

/// The result of a successful free `call_workflow` invocation.
///
/// For paid workflows, `call_workflow` returns
/// [`crate::error::Error::X402Unpaid`] instead of this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallWorkflowResult {
    /// Unique execution ID (pollable via `get_execution` for status).
    pub execution_id: String,

    /// Execution status. For free read workflows, typically `"success"`.
    pub status: String,

    /// The workflow's output. Shape is workflow-specific.
    pub output: serde_json::Value,

    /// Optional ERC-8004 feedback prompt (KeeperHub auto-registers
    /// executed workflows on the 8004scan reputation registry).
    #[serde(default)]
    pub feedback: Option<serde_json::Value>,
}

/// Optional filters for [`crate::mcp::McpClient::search_workflows`].
///
/// All fields are `None` by default. Build with struct-update syntax so
/// un-set fields are omitted from the MCP request payload (the server
/// only filters on a field if it's present in the args object).
///
/// ```no_run
/// use keeperhub_rs::types::SearchWorkflowsOptions;
/// let opts = SearchWorkflowsOptions {
///     query: Some("morpho health".into()),
///     category: Some("defi".into()),
///     ..Default::default()
/// };
/// # let _ = opts;
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchWorkflowsOptions {
    /// Natural-language search query. Matches against `name` and
    /// `description` (server-side behavior; this client just forwards
    /// the string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Category filter (e.g. `"defi"`, `"monitoring"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Chain ID filter (e.g. `"1"` for Ethereum, `"8453"` for Base).
    /// Always a string per the API — the same shape used by the
    /// `web3/*` actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,

    /// Workflow type filter: `"read"` (executes and returns the result)
    /// or `"write"` (returns unsigned calldata for the caller to submit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_type: Option<String>,

    /// Sort order. `"popular"` ranks by call count, `"recent"` by
    /// listing date. Omit for the server's default catalog ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

/// Optional metadata for [`crate::mcp::McpClient::create_workflow`].
///
/// All fields are `None` by default. Use struct-update syntax to set
/// only the ones you care about.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowOptions {
    /// Workflow description. Optional per the MCP schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether to enable the workflow at creation time. Defaults to
    /// `false` on the server. Manual triggers can be executed
    /// regardless of `enabled`; the flag matters for scheduled /
    /// event / block / webhook triggers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Project ID to assign the workflow to. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Tag ID to label the workflow. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<String>,
}

/// Optional metadata to attach when publishing a workflow to the
/// marketplace via [`crate::mcp::McpClient::list_workflow`].
///
/// On first publish, `slug` is required and the server refuses to let
/// you change it later. On re-publish, the slug is preserved. The
/// `workflow_type` must be `"read"` for read-only workflows (most of
/// the marketplace) or `"write"` for state-changing ones.
///
/// `input_schema` and `output_mapping` are the public-facing JSON
/// Schemas that marketplace callers see in their MCP tool descriptors;
/// populate them carefully because they shape how agents invoke the
/// workflow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkflowOptions {
    /// Public URL slug (e.g. `"aave-v3-risk-check"`). Required on
    /// first publish; preserved on re-publish. Permanent once set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,

    /// Workflow category (e.g. `"defi"`, `"monitoring"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Chain ID this workflow targets (e.g. `"1"` for Ethereum,
    /// `"8453"` for Base). Omit for multi-chain workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,

    /// JSON Schema describing the workflow's input parameters.
    /// Required for listed workflows so marketplace callers know the
    /// shape to pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,

    /// Mapping from public output field names to the workflow's
    /// internal node outputs. Required for listed workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_mapping: Option<serde_json::Value>,

    /// Workflow type: `"read"` (executes and returns the result) or
    /// `"write"` (returns unsigned calldata for the caller to submit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_type: Option<String>,
}

/// Fields for [`crate::mcp::McpClient::update_workflow`].
///
/// All fields are optional — only those that are `Some` are sent in
/// the request payload. `enabled = false` is the typical way to stop
/// a scheduled workflow from firing without deleting it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowUpdate {
    /// New workflow name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// New workflow description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Replace the entire node graph. None means "leave nodes
    /// unchanged". To set just one field, you must resend the full
    /// graph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<Node>>,

    /// Replace the entire edge set. None means "leave edges unchanged".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<Edge>>,

    /// Set enabled state. None means "leave enabled state unchanged".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Project ID to assign. `Some("")` is treated as null by the
    /// server (use the `null` JSON literal to unassign). Wrap with
    /// care.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Tag ID to assign.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<String>,
}

/// Fields for [`crate::mcp::McpClient::update_workflow_listing`].
///
/// All fields are optional. The server's quirk: **`priceUsdcPerCall`
/// is only honored while the workflow is unlisted**. To change the
/// price of a listed workflow, call `unlist_workflow` first, then
/// `update_workflow_listing` with the new price, then `list_workflow`
/// again. The same dance is required for the UI's "Marketplace" button.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingUpdate {
    /// Updated category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Updated chain ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,

    /// Updated input JSON Schema (full replace, not merge).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,

    /// Updated output mapping (full replace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_mapping: Option<serde_json::Value>,

    /// Updated workflow type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_type: Option<String>,

    /// Updated price in USDC (only honored while unlisted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_usdc_per_call: Option<String>,
}
