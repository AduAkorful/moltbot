# keeperhub-rs

> First Rust client for the [KeeperHub](https://keeperhub.com) onchain
> automation platform.

KeeperHub ships TypeScript and Python adapters; `keeperhub-rs` fills the
Rust gap. It targets the [KeeperHub MCP server](https://docs.keeperhub.com/ai-tools/mcp-server)
(JSON-RPC over HTTP) and adds typed helpers for the high-traffic
plugins (Aave V3, Morpho Blue) plus pre-built marketplace workflow
templates.

## Status

Pre-1.0 but functional. All advertised methods work against the real
KeeperHub MCP server. Coverage of the 31-tool MCP surface is growing;
the table below lists what's wired up.

## Crate map

| Module | Purpose |
|---|---|
| [`mcp`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/mcp/) | `McpClient` — JSON-RPC over HTTP, JWT session, content-envelope unwrap, x402 detection |
| [`aave`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/aave/) | Typed `AaveV3::supply` / `withdraw` / `get_user_account_data` over `execute_protocol_action` |
| [`morpho`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/morpho/) | Typed `Morpho::get_position` / `get_market` / `get_market_params` + `compute_health_factor` (pure Rust) |
| [`workflows`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/workflows/) | Pre-built workflow blueprints (e.g. `aave_v3_risk_check()`) for `create_workflow` + `list_workflow` |
| [`x402`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/x402/) | EIP-3009 `TransferWithAuthorization` builder + 402-challenge parser |
| [`types`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/types/) | `Workflow`, `ExecutionDetail`, `SearchWorkflowsOptions`, `CreateWorkflowOptions`, etc. |
| [`error`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/error/) | `Error` enum with `Http` / `Api` / `X402Unpaid` / `Config` / `Crypto` / `Serde` / `Mcp` / `Internal` |
| [`rest`](https://docs.rs/keeperhub-rs/latest/keeperhub_rs/rest/) | REST stub (not yet implemented — MCP covers all needs) |

## Quickstart

```rust,no_run
use keeperhub_rs::mcp::McpClient;
use keeperhub_rs::aave::AaveV3;
use keeperhub_rs::types::SearchWorkflowsOptions;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), keeperhub_rs::Error> {
    let client = McpClient::new("https://app.keeperhub.com/mcp", "kh_your_key");

    // 1. Search the marketplace for Aave workflows.
    let defi = client
        .search_workflows(SearchWorkflowsOptions {
            category: Some("defi".into()),
            ..Default::default()
        })
        .await?;
    for w in &defi { println!("  - {} ({})", w.name, w.listed_slug.as_deref().unwrap_or("-")); }

    // 2. Call our own published workflow (the example Aave V3 risk check).
    let _ = client.call_workflow("aave-v3-risk-check", json!({
        "wallet": "0x54F9Fe5A1f63064fc083928df60A95db2dc2CE39"
    })).await?;

    // 3. Read Aave V3 account data directly via execute_protocol_action.
    let _data = AaveV3::get_user_account_data(
        &client, "1", "0x54F9Fe5A1f63064fc083928df60A95db2dc2CE39"
    ).await?;

    Ok(())
}
```

## Examples

Run any of these with `KEEPERHUB_API_KEY=kh_... cargo run --example <name>`:

| Example | What it does |
|---|---|
| `list_workflows` | Lists all workflows in your org |
| `call_workflow` | Calls a marketplace workflow by slug (free) |
| `search_workflows` | Searches the marketplace with filters |
| `aave_supply` | Supplies USDC to Aave V3 (or reads account data with `AAVE_DRY_RUN=1`) |
| `morpho_position` | Reads a Morpho Blue position and computes its health factor |
| `publish_aave_risk_check` | Publishes the Aave V3 portfolio risk check workflow to the marketplace |

## Live integration tests

`cargo test` runs the unit tests (no network). The live tests against
the real KeeperHub MCP server are gated behind the `live-mcp` feature
flag and require `KEEPERHUB_API_KEY`:

```sh
KEEPERHUB_API_KEY=kh_... cargo test --features live-mcp --test live_mcp -- --test-threads=1
```

The live tests are read-only (they create, list, and immediately
unlist a single test workflow; no onchain txs).

## Development

```sh
cargo check                # type check
cargo clippy --all-targets --all-features -- -D warnings   # lint
cargo test                 # unit tests (no network)
cargo doc --no-deps        # build docs locally
cargo package -p keeperhub-rs --allow-dirty --no-verify --list   # verify what's in the published crate
```

## License

MIT — see [LICENSE](LICENSE).

## See also

- [KeeperHub docs](https://docs.keeperhub.com)
- [KeeperHub MCP server docs](https://docs.keeperhub.com/ai-tools/mcp-server)
- [MoltBot project plan](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)
