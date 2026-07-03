//! Example: publish the "Aave V3 Portfolio Risk Check" workflow to
//! the KeeperHub marketplace.
//!
////! Walks through the full publish flow:
//!   1. Build the workflow definition (from `keeperhub_rs::workflows`)
//!   2. `create_workflow` in the org
//!   3. `update_workflow` to set `enabled: true` (cosmetic for our
//!      manual-trigger workflow but recommended)
//!   4. `list_workflow` to publish to the marketplace (slug, category,
//!      chain, input schema, output mapping, workflow type)
//!
//! **The price cannot be set in this call.** The KeeperHub
//! `list_workflow` MCP tool does not accept `priceUsdcPerCall`. After
//! this example prints the workflow ID, you have two options:
//!
//!   a. **UI path (recommended for one-off):** open the workflow in
//!      the KeeperHub app, click "Marketplace", set the price to
//!      $0.01, save. The app's price UI is the canonical path.
//!
//!   b. **Programmatic path:** set the env var `AAVE_RISK_CHECK_SET_PRICE=1`
//!      and the example will additionally call `set_listing_price`
//!      (which unlists, updates the price, and re-lists in one shot).
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! cargo run --example publish_aave_risk_check
//! ```
//!
//! Or with the programmatic price path:
//! ```sh
//! cargo run --example publish_aave_risk_check -- --set-price
//! ```
//!
//! Optional: `RUST_LOG=keeperhub_rs=debug cargo run --example publish_aave_risk_check`
//! to see per-call tracing.

use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use keeperhub_rs::types::{CreateWorkflowOptions, ListWorkflowOptions};
use keeperhub_rs::workflows::{
    aave_v3_risk_check, aave_v3_risk_check_input_schema, aave_v3_risk_check_output_mapping,
    AAVE_V3_RISK_CHECK_CATEGORY, AAVE_V3_RISK_CHECK_CHAIN, AAVE_V3_RISK_CHECK_PRICE, AAVE_V3_RISK_CHECK_SLUG,
};
use keeperhub_rs::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let set_price = std::env::args().any(|a| a == "--set-price")
        || std::env::var("AAVE_RISK_CHECK_SET_PRICE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    let api_key = std::env::var("KEEPERHUB_API_KEY").map_err(|_| {
        keeperhub_rs::Error::Config(
            "KEEPERHUB_API_KEY environment variable not set. \
             See plans/setup-verified.md for how to get one."
                .to_string(),
        )
    })?;

    let client = McpClient::new(DEFAULT_MCP_URL, &api_key);

    // 1. Build the workflow definition.
    let (name, description, nodes, edges) = aave_v3_risk_check();
    let description = description.map(|s| s.to_string());
    tracing::info!(
        name = %name,
        nodes = nodes.len(),
        edges = edges.len(),
        "built Aave V3 risk check workflow"
    );

    // 2. Create it in the org. New workflows are enabled=false by
    //    default; for our manual trigger it doesn't strictly matter,
    //    but we set it true for consistency.
    let created = client
        .create_workflow(
            name,
            nodes,
            edges,
            CreateWorkflowOptions {
                description,
                enabled: Some(true),
                project_id: None,
                tag_id: None,
            },
        )
        .await?;
    let workflow_id = created.id.clone();
    tracing::info!(
        workflow_id = %workflow_id,
        workflow_name = %created.name,
        "workflow created"
    );

    // 3. Publish to the marketplace. The slug is permanent on first
    //    publish; re-publishing preserves it.
    let listed = client
        .list_workflow(
            &workflow_id,
            ListWorkflowOptions {
                slug: Some(AAVE_V3_RISK_CHECK_SLUG.to_string()),
                category: Some(AAVE_V3_RISK_CHECK_CATEGORY.to_string()),
                chain: Some(AAVE_V3_RISK_CHECK_CHAIN.to_string()),
                input_schema: Some(aave_v3_risk_check_input_schema()),
                output_mapping: Some(aave_v3_risk_check_output_mapping()),
                workflow_type: Some("read".to_string()),
            },
        )
        .await?;
    tracing::info!(
        workflow_id = %workflow_id,
        slug = ?listed.listed_slug,
        is_listed = listed.is_listed,
        "workflow listed in marketplace"
    );

    // 4. Optionally set the price programmatically (the
    //    unlist/update_listing/relist dance).
    if set_price {
        tracing::warn!(
            "AAVE_RISK_CHECK_SET_PRICE=1: setting price to ${} via unlist/update_listing/relist dance",
            AAVE_V3_RISK_CHECK_PRICE
        );
        let _ = client
            .set_listing_price(&workflow_id, AAVE_V3_RISK_CHECK_PRICE)
            .await?;
        tracing::info!(price = AAVE_V3_RISK_CHECK_PRICE, "price set");
    } else {
        println!();
        println!("================ NEXT STEP (UI) ================");
        println!("The price is NOT yet set (the MCP publish path doesn't accept it).");
        println!("Open the workflow in the KeeperHub app:");
        println!("  https://app.keeperhub.com");
        println!("Find workflow id {workflow_id} ({name}) and click the \"Marketplace\"");
        println!("button. Set the price to ${} per call. Save.", AAVE_V3_RISK_CHECK_PRICE);
        println!("================================================");
    }

    println!();
    println!("workflow_id  = {workflow_id}");
    println!("slug         = {}", AAVE_V3_RISK_CHECK_SLUG);
    println!("price        = ${} per call", AAVE_V3_RISK_CHECK_PRICE);
    println!("chain        = {} (Ethereum mainnet)", AAVE_V3_RISK_CHECK_CHAIN);
    println!("category     = {}", AAVE_V3_RISK_CHECK_CATEGORY);
    println!();
    println!("Test it with:");
    println!("  SEARCH_QUERY={} cargo run -p keeperhub-rs --example search_workflows", AAVE_V3_RISK_CHECK_SLUG);
    println!("  WORKFLOW_SLUG={} cargo run -p keeperhub-rs --example call_workflow", AAVE_V3_RISK_CHECK_SLUG);

    Ok(())
}
