# keeperhub-rs

> Rust client for the KeeperHub onchain automation platform.

This crate is the **first Rust client for KeeperHub** and fills a real gap in their ecosystem (they ship TypeScript and Python adapters; no Rust).

## Status

🚧 **Pre-alpha.** The crate compiles and has module skeletons, but no real logic yet. The actual MCP client, x402 signer, and REST API client land in Phase 4.4+ of the project plan.

## Planned surface

```rust,no_run
use keeperhub_rs::prelude::*;

let client = McpClient::new("https://app.keeperhub.com/mcp", api_key);

// List workflows
let workflows = client.list_workflows().await?;

// Call a workflow (free or paid)
let result = client.call_workflow("aave-v3-supply", json!({
    "asset": "USDC",
    "amount": "100",
})).await?;

// Pay for a paid workflow via x402
let paid = client.call_paid_workflow("price-feed-eth", json!({})).await?;
```

## Modules

| Module | Purpose |
|---|---|
| `mcp` | MCP server client over HTTP (JSON-RPC) |
| `rest` | REST API client (workflows, executions, analytics) |
| `x402` | EIP-3009 `TransferWithAuthorization` builder + 402 auto-pay |
| `types` | Shared types (Workflow, Execution, Run, etc.) |
| `error` | Error types |

## See also

- [Project research](../plans/moltbot-deep-research.md)
- [KeeperHub docs summary](../plans/keeperhub-docs-summary.md)
- [Setup checklist](../plans/setup-verified.md)
