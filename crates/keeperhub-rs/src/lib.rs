//! Rust client for the KeeperHub onchain automation platform.
//!
//! `keeperhub-rs` is the first Rust-native client for KeeperHub. It exposes
//! three primary surfaces:
//!
//! - [`mcp`] — Model Context Protocol client for talking to KeeperHub's
//!   hosted MCP server (`https://app.keeperhub.com/mcp`). Used by agents
//!   to discover and call workflows.
//! - [`rest`] — REST API client for direct workflow management and analytics.
//! - [`x402`] — EIP-3009 `TransferWithAuthorization` builder and the
//!   402 auto-pay loop that lets agents pay for paid workflows in USDC on
//!   Base (or USDC.e on Tempo).
//!
//! # Status
//!
//! Pre-alpha. The module skeletons are in place; the real logic lands in
//! Phase 4.4+ of the project plan.
//!
//! # Example
//!
//! ```no_run
//! use keeperhub_rs::mcp::McpClient;
//!
//! # async fn demo() -> keeperhub_rs::Result<()> {
//! let client = McpClient::new("https://app.keeperhub.com/mcp", "kh_...");
//! // list_workflows, call_workflow, etc. land in the next phase.
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod mcp;
pub mod rest;
pub mod types;
pub mod x402;

pub use error::{Error, Result};
pub use mcp::McpClient;

/// Re-exports of common types for ergonomic use.
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::mcp::McpClient;
    pub use crate::types::{Execution, ExecutionStatus, Workflow};
}
