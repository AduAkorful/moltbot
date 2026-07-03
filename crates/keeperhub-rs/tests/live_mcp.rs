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
