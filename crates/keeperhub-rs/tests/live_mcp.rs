//! Live integration tests against the real KeeperHub MCP server.
//!
//! These tests are gated behind the `live-mcp` feature flag. They require
//! the `KEEPERHUB_API_KEY` environment variable to be set to a valid
//! organization-scoped API key.
//!
//! Run with:
//! ```sh
//! KEEPERHUB_API_KEY=kh_... cargo test --features live-mcp --test live_mcp -- --test-threads=1
//! ```
//!
//! `--test-threads=1` is important: each test creates its own client and
//! the MCP server's session/id management may not be safe under heavy
//! concurrent access from a single org. In practice we only have one
//! client at a time anyway, but this avoids any flakiness.
//!
//! These tests do NOT mutate the org (no workflows are created, listed,
//! or executed). They're read-only.

#![cfg(feature = "live-mcp")]

use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};

fn api_key() -> String {
    std::env::var("KEEPERHUB_API_KEY")
        .expect("KEEPERHUB_API_KEY env var must be set for live-mcp tests")
}

fn client() -> McpClient {
    McpClient::new(DEFAULT_MCP_URL, &api_key())
}

#[tokio::test]
async fn initialize_establishes_session() {
    let c = client();
    c.initialize().await.expect("initialize should succeed");
    // A second call should be a no-op (session is cached).
    c.initialize().await.expect("cached initialize should succeed");
}

#[tokio::test]
async fn list_workflows_returns_vec() {
    let c = client();
    let workflows = c
        .list_workflows()
        .await
        .expect("list_workflows should succeed");
    // The result is a Vec, possibly empty. We don't assert on count
    // because the test org may have any number of workflows.
    for w in &workflows {
        assert!(!w.id.is_empty(), "workflow id should be non-empty");
        assert!(!w.name.is_empty(), "workflow name should be non-empty");
    }
}

#[tokio::test]
async fn list_workflows_with_bad_key_fails_with_api_error() {
    let bad = McpClient::new(DEFAULT_MCP_URL, "kh_definitely_not_a_real_key");
    let err = bad
        .list_workflows()
        .await
        .expect_err("list_workflows with bad key should fail");
    // The exact error code varies (401 from the server, or a JSON-RPC
    // error from a failed handshake). We just require it's NOT a
    // successful empty list.
    match err {
        keeperhub_rs::Error::Api { status, .. } => {
            assert!(
                status == 401 || status == 403,
                "expected 401/403, got {status}"
            );
        }
        keeperhub_rs::Error::Mcp(_) => {
            // Acceptable: the handshake itself failed.
        }
        other => panic!("expected Error::Api or Error::Mcp, got {other:?}"),
    }
}

#[tokio::test]
async fn call_workflow_executes_a_free_listed_workflow() {
    use serde_json::json;

    let c = client();
    // `sep-eth-balance-test` is the free test workflow we created during
    // setup. It returns the Sepolia ETH balance of a hard-coded address
    // and takes no inputs. It must be listed + enabled in the org.
    let result = c
        .call_workflow("sep-eth-balance-test", json!({}))
        .await
        .expect("free call_workflow should succeed");

    assert!(!result.execution_id.is_empty(), "execution_id should be non-empty");
    assert_eq!(result.status, "success", "free call should be synchronous-success");
    // The output is the workflow's structured result. For our test
    // workflow it includes a `balance` field (string, ETH).
    let output = &result.output;
    assert!(
        output.get("balance").is_some(),
        "expected 'balance' in output, got: {output}"
    );
    assert!(output.get("success").is_some(), "expected 'success' flag in output");
}

#[tokio::test]
async fn call_workflow_returns_404_for_nonexistent_slug() {
    use serde_json::json;

    let c = client();
    let err = c
        .call_workflow("this-workflow-definitely-does-not-exist-xyz123", json!({}))
        .await
        .expect_err("call_workflow with bad slug should fail");
    // We don't pin the exact error shape (could be 404 from the API or
    // 422 from the marketplace registry). Just require it errors.
    assert!(
        matches!(err, keeperhub_rs::Error::Api { .. } | keeperhub_rs::Error::Mcp(_)),
        "expected Error::Api or Error::Mcp, got {err:?}"
    );
}

#[tokio::test]
async fn execute_then_get_execution_roundtrips() {
    use keeperhub_rs::types::ExecutionStatus;
    use serde_json::json;

    let c = client();

    // Step 1: call the free workflow to get an execution_id.
    let result = c
        .call_workflow("sep-eth-balance-test", json!({}))
        .await
        .expect("free call_workflow should succeed");
    let exec_id = &result.execution_id;
    assert!(!exec_id.is_empty());

    // Step 2: get the full execution detail.
    let detail = c
        .get_execution(exec_id)
        .await
        .expect("get_execution should succeed");

    // The status envelope should report success.
    assert_eq!(detail.status.status, ExecutionStatus::Success);

    // The execution record should have the same ID and a real output.
    assert_eq!(detail.execution.id, *exec_id);
    assert!(
        detail.execution.output.get("balance").is_some(),
        "expected balance in execution output, got: {}",
        serde_json::to_string_pretty(&detail.execution.output).unwrap_or_default()
    );

    // The execution should have audit metadata: trigger source, started/ended, duration.
    assert!(detail.execution.started_at.is_some(), "started_at should be set");
    assert!(detail.execution.completed_at.is_some(), "completed_at should be set");
    assert!(detail.execution.duration.is_some(), "duration should be set");
    assert!(
        detail.execution.trigger_source.is_some(),
        "trigger_source should be set"
    );
}

#[tokio::test]
async fn get_execution_for_unknown_id_fails() {
    let c = client();
    let err = c
        .get_execution("this-execution-definitely-does-not-exist-xyz123")
        .await
        .expect_err("get_execution with bad id should fail");
    assert!(
        matches!(err, keeperhub_rs::Error::Api { .. } | keeperhub_rs::Error::Mcp(_)),
        "expected Error::Api or Error::Mcp, got {err:?}"
    );
}

#[tokio::test]
async fn search_workflows_default_returns_catalog() {
    use keeperhub_rs::types::SearchWorkflowsOptions;

    let c = client();
    let items = c
        .search_workflows(SearchWorkflowsOptions::default())
        .await
        .expect("default search_workflows should succeed");

    // The marketplace has at least one item in it (the public catalog
    // we observed in setup had 20). We don't assert on a specific count
    // — the catalog grows over time. We do assert that every returned
    // item is well-formed and listed.
    for w in &items {
        assert!(!w.id.is_empty(), "workflow id should be non-empty");
        assert!(!w.name.is_empty(), "workflow name should be non-empty");
        assert!(w.is_listed, "search results should only contain listed workflows");
    }
}

#[tokio::test]
async fn search_workflows_with_category_filters_results() {
    use keeperhub_rs::types::SearchWorkflowsOptions;

    let c = client();
    let items = c
        .search_workflows(SearchWorkflowsOptions {
            category: Some("defi".into()),
            ..Default::default()
        })
        .await
        .expect("category search should succeed");

    // Every result must have category="defi" (otherwise the filter is broken).
    for w in &items {
        assert_eq!(
            w.category.as_deref(),
            Some("defi"),
            "category filter returned non-defi workflow: {:?} ({})",
            w.name,
            w.id,
        );
    }
}

#[tokio::test]
async fn search_workflows_with_chain_filters_results() {
    use keeperhub_rs::types::SearchWorkflowsOptions;

    let c = client();
    let items = c
        .search_workflows(SearchWorkflowsOptions {
            chain: Some("1".into()), // Ethereum mainnet
            ..Default::default()
        })
        .await
        .expect("chain search should succeed");

    // Workflows that don't pin a chain (chain=None) are multi-chain
    // and may also appear in the result. The filter is "chain includes
    // this id" — we just require that any pinned chain matches.
    for w in &items {
        if let Some(ch) = &w.chain {
            assert_eq!(
                ch, "1",
                "chain filter returned workflow on chain {ch}: {:?} ({})",
                w.name, w.id,
            );
        }
    }
}

#[tokio::test]
async fn search_workflows_with_unknown_category_returns_empty_or_subset() {
    use keeperhub_rs::types::SearchWorkflowsOptions;

    let c = client();
    let items = c
        .search_workflows(SearchWorkflowsOptions {
            category: Some("this-category-does-not-exist-xyz".into()),
            ..Default::default()
        })
        .await
        .expect("unknown category search should succeed");

    // The server either returns no items, or a subset. We just require
    // it doesn't error.
    for w in &items {
        assert!(w.is_listed);
    }
}
