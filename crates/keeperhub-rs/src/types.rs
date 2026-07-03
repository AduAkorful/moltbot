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
    /// Completed successfully (alias returned by some endpoints).
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
}

/// A single execution of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    /// Unique execution ID.
    pub id: String,

    /// The workflow that was executed.
    pub workflow_id: String,

    /// Current status.
    pub status: ExecutionStatus,

    /// When the execution started.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,

    /// When the execution ended (if terminal).
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,

    /// Transaction hashes produced by this execution (for web3 actions).
    #[serde(default)]
    pub tx_hashes: Vec<String>,

    /// Total gas used (in wei) across all transactions.
    #[serde(default)]
    pub gas_used: Option<String>,
}

/// Detailed logs for an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionLogs {
    /// The execution these logs belong to.
    pub execution_id: String,

    /// Structured log entries (one per node).
    pub entries: Vec<LogEntry>,
}

/// A single log entry in an execution's audit trail.
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
