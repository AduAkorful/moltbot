//! Example: list all workflows in your KeeperHub organization.
//!
//! Demonstrates the full happy path: construct an [`McpClient`], perform
//! the MCP handshake, call `list_workflows`, parse the response, and
//! print a summary.
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! cargo run --example list_workflows
//! ```
//!
//! Optional: `RUST_LOG=keeperhub_rs=debug cargo run --example list_workflows`
//! to see per-call tracing.

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

    let workflows = client.list_workflows().await?;
    tracing::info!(count = workflows.len(), "workflows listed");

    if workflows.is_empty() {
        println!("(no workflows in this org yet)");
    } else {
        for w in &workflows {
            let price = w
                .price_usdc_per_call
                .as_deref()
                .map(|p| format!(" (${p}/call)"))
                .unwrap_or_default();
            let listed = if w.is_listed { " [listed]" } else { "" };
            println!(
                "- {} (id={}){}{}  {}",
                w.name,
                w.id,
                price,
                listed,
                w.description.as_deref().unwrap_or(""),
            );
        }
    }

    Ok(())
}
