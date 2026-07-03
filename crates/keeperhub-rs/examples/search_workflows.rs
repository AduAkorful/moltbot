//! Example: search the KeeperHub marketplace for listed workflows.
//!
//! Demonstrates [`McpClient::search_workflows`] with several filters.
//! Prints a table of the matching workflows with their slug, price,
//! category, and chain.
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! cargo run --example search_workflows
//! ```
//!
//! Optional environment overrides:
//! - `SEARCH_QUERY`     — free-text query (default: none, returns full catalog)
//! - `SEARCH_CATEGORY`  — category filter (e.g. `defi`, `monitoring`)
//! - `SEARCH_CHAIN`     — chain ID filter (e.g. `1`, `8453`)
//! - `SEARCH_TYPE`      — workflow type (`read` or `write`)
//!
//! ```sh
//! SEARCH_CATEGORY=defi SEARCH_CHAIN=1 cargo run --example search_workflows
//! ```
//!
//! Optional: `RUST_LOG=keeperhub_rs=debug cargo run --example search_workflows`
//! to see per-call tracing.

use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use keeperhub_rs::types::SearchWorkflowsOptions;
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

    let opts = SearchWorkflowsOptions {
        query: std::env::var("SEARCH_QUERY").ok(),
        category: std::env::var("SEARCH_CATEGORY").ok(),
        chain: std::env::var("SEARCH_CHAIN").ok(),
        workflow_type: std::env::var("SEARCH_TYPE").ok(),
        sort: std::env::var("SEARCH_SORT").ok(),
    };

    tracing::info!(?opts, "searching marketplace");

    let client = McpClient::new(DEFAULT_MCP_URL, &api_key);
    let items = client.search_workflows(opts).await?;

    tracing::info!(count = items.len(), "search returned");

    if items.is_empty() {
        println!("(no workflows matched)");
    } else {
        println!(
            "{:<46}  {:<14}  {:<12}  {:<8}  price ($/call or 'free')",
            "slug", "name (truncated)", "category", "chain"
        );
        println!("{}", "-".repeat(96));
        for w in &items {
            let slug = w.listed_slug.as_deref().unwrap_or("(no slug)");
            let name_short = if w.name.len() > 14 {
                format!("{}…", &w.name[..13])
            } else {
                w.name.clone()
            };
            let category = w.category.as_deref().unwrap_or("-");
            let chain = w.chain.as_deref().unwrap_or("-");
            let price = w
                .price_usdc_per_call
                .as_deref()
                .map(|p| format!("${p}/call"))
                .unwrap_or_else(|| "free".to_string());
            println!("{slug:<46}  {name_short:<14}  {category:<12}  {chain:<8}  {price}");
        }
    }

    Ok(())
}
