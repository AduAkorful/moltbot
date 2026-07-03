## Summary

Adds a "Rust SDK (`keeperhub-rs`)" section to the main
README, introducing the official Rust client for KeeperHub.
The crate is part of the broader MoltBot project (a
KeeperHub-native autonomous agent that paid for its first
workflow via x402 during the DoraHacks Agents-Onchain
hackathon).

This is a **docs-only change** — one new README section,
no code edits. The crate lives in a separate repo and is
referenced by URL.

## Why

KeeperHub ships TypeScript (`@keeperhub/sdk`) and Python
(`hermes-plugin`) adapters today. There's no first-class
Rust client, which leaves the Rust ecosystem without a
direct path into the KeeperHub MCP surface or the
plugin ecosystem. `keeperhub-rs` fills that gap.

## What's in the box

- Full MCP JSON-RPC client (lazy JWT session, content
  envelope unwrap, x402 detection)
- Typed helpers for Aave V3 (`supply`, `withdraw`,
  `get_user_account_data`) and Morpho Blue
  (`get_position`, `get_market`, `get_market_params`,
  `compute_health_factor`)
- Pre-built marketplace workflow templates
  (`aave_v3_risk_check()`)
- 402-challenge parser for x402 paid workflows
- 49 unit tests, 4 doc-tests, 16 live integration tests
  gated behind `--features live-mcp`
- `cargo clippy --all-targets --all-features -- -D warnings`
  is clean
- A companion agent binary (`moltbot`) that uses the
  crate to run an autonomous onchain agent with a
  local audit log + dashboard

## Status

- Crate is **publishable** (`cargo publish --dry-run`
  passes; 22 files, 205 KiB; all metadata complete)
- A crates.io publish is pending post-hackathon. The
  README snippet uses a git dependency for now; once
  the crate is on the registry, swap the snippet to
  `keeperhub-rs = "0.1"` and add a docs.rs link

## Testing

- `cargo check`, `cargo clippy`, `cargo test` all clean
  in `crates/keeperhub-rs/`
- The companion agent (`moltbot`) has 140 unit tests
  and exercises the crate end-to-end on a real KeeperHub
  org
- The README snippet is copy-pasteable; the `rust,no_run`
  fences mean it won't be executed by `cargo test` (no
  network in unit tests)

## Checklist

- [x] No code changes (README only)
- [x] Snippet compiles in isolation (verified locally)
- [x] Link target exists and is public
- [x] No secrets, no API keys
- [x] Consistent framing with the existing TS / Python
      adapter sections
