//! Example: call a free marketplace workflow by slug.
//!
//! Demonstrates the happy path for `McpClient::call_workflow`. For
//! paid workflows, the call returns `Error::X402Unpaid` with the
//! challenge details — the caller should then use the KeeperHub
//! agentic wallet MCP for auto-pay.
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! cargo run --example call_workflow
//! ```
//!
//! Optional: `RUST_LOG=keeperhub_rs=debug cargo run --example call_workflow`
//! to see per-call tracing.

use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use keeperhub_rs::Result;
use serde_json::json;

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

    // `sep-eth-balance-test` is the free test workflow created during
    // setup. It returns the Sepolia ETH balance of a hard-coded address
    // and takes no inputs.
    let slug = std::env::var("WORKFLOW_SLUG").unwrap_or_else(|_| "sep-eth-balance-test".into());

    tracing::info!(%slug, "calling workflow");
    match client.call_workflow(&slug, json!({})).await {
        Ok(result) => {
            tracing::info!(
                execution_id = %result.execution_id,
                status = %result.status,
                "workflow completed"
            );
            println!("status:  {}", result.status);
            println!("exec id: {}", result.execution_id);
            println!("output:  {}", serde_json::to_string_pretty(&result.output).unwrap());
            if let Some(fb) = &result.feedback {
                println!("feedback prompt: {}", fb.get("prompt").and_then(|p| p.as_str()).unwrap_or(""));
            }
        }
        Err(keeperhub_rs::Error::X402Unpaid { slug, challenge }) => {
            tracing::warn!(%slug, ?challenge, "workflow is paid; use the agentic wallet MCP to auto-pay");
            println!("PAID WORKFLOW: {slug}");
            println!("challenge: {challenge}");
            println!("hint: use mcp__plugin_keeperhub_wallet__call_workflow for auto-pay");
        }
        Err(e) => {
            tracing::error!(error = %e, "call_workflow failed");
            return Err(e);
        }
    }

    Ok(())
}
