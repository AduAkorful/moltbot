//! Shared types used across the KeeperHub Rust client.
//!
//! These types map closely to the JSON shapes returned by the KeeperHub
//! REST API and MCP server. They are intentionally minimal in the
//! pre-alpha scaffold; richer fields will be added as the corresponding
//! modules are implemented.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A KeeperHub workflow.
///
/// Workflows are the unit of automation. Each workflow has triggers,
/// actions, and conditions. The struct here is a summary view; the
/// full graph (nodes + edges) is fetched via [`crate::rest::RestClient::get_workflow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Optional price per execution in USD, if the workflow is published
    /// to the marketplace as a paid workflow.
    #[serde(default)]
    pub price_usd: Option<f64>,

    /// When the workflow was created.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// When the workflow was last updated.
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
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
            Self::Completed | Self::Failed | Self::Cancelled
        )
    }
}

/// A single execution of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    /// Unique execution ID.
    pub id: Uuid,

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
///
/// This is a thin wrapper for now; the real shape will be defined when
/// the audit-trail renderer is implemented in MoltBot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogs {
    /// The execution these logs belong to.
    pub execution_id: Uuid,

    /// Structured log entries (one per node).
    pub entries: Vec<LogEntry>,
}

/// A single log entry in an execution's audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
