//! Example: list all workflows in your KeeperHub organization.
//!
//! This example demonstrates how to construct an [`McpClient`] and call
//! the `list_workflows` tool. The actual call lands in the next phase
//! of the project; for now, the example verifies the client compiles
//! and the environment is set up.
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! cargo run --example list_workflows
//! ```
//!
//! The example will print a message indicating the API key was found
//! and the client was constructed. When the real `list_workflows`
//! implementation lands, it will print the list of workflows.

use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use keeperhub_rs::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let api_key = std::env::var("KEEPERHUB_API_KEY").map_err(|_| {
        keeperhub_rs::Error::Config(
            "KEEPERHUB_API_KEY environment variable not set. \
             See plans/setup-verified.md for how to get one."
                .to_string(),
        )
    })?;

    let client = McpClient::new(DEFAULT_MCP_URL, &api_key);

    tracing::info!(url = %client.url(), "MCP client constructed");

    match client.list_workflows().await {
        Ok(workflows) => {
            tracing::info!(count = workflows.len(), "workflows listed");
            for w in workflows {
                println!("- {} ({}) — {}", w.name, w.id, w.description.unwrap_or_default());
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "list_workflows not yet implemented (expected in pre-alpha)");
        }
    }

    Ok(())
}
