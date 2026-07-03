# MoltBot

> First paying customer of the KeeperHub marketplace. A Rust autonomous agent that funds itself by paying onchain keepers via x402, earning yield on idle USDC, and auditing every economic decision through KeeperHub's audit trail.

**Competition:** [KeeperHub — Agents Onchain Hackathon](https://dorahacks.io/hackathon/agents-onchain/detail) (DoraHacks)
**Build window:** Jul 27 → Aug 13, 2026
**Pre-build starts:** Jul 3, 2026

## Project layout

```
moltbot/
├── README.md                              ← you are here
├── .gitignore
├── Cargo.toml                             ← workspace root
├── crates/
│   ├── keeperhub-rs/                      ← Rust client for KeeperHub (the bounty deliverable)
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs
│   │   │   ├── types.rs
│   │   │   ├── mcp.rs                     ← MCP client (stub)
│   │   │   ├── rest.rs                    ← REST client (stub)
│   │   │   └── x402.rs                    ← x402 auto-pay (stub)
│   │   └── examples/
│   │       └── list_workflows.rs
│   └── moltbot/                           ← the agent binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs                    ← placeholder entry
└── plans/
    ├── moltbot-deep-research.md           ← 15 sections, 803 lines
    ├── keeperhub-docs-summary.md          ← quick reference card
    └── setup-verified.md                  ← local environment checklist
```

## Where we are

- [x] Phase 1 — Context & constraints (hackathon analyzed)
- [x] Phase 2 — Ideation & scoring (MoltBot chosen, 3 directions compared)
- [x] Phase 3 — Deep product research (15 sections)
- [x] Phase 4.1 — KeeperHub docs summarized (`plans/keeperhub-docs-summary.md`)
- [x] Phase 4.2 — Setup checklist written (`plans/setup-verified.md`)
- [x] Phase 4.3 — Rust workspace scaffolded (this commit)
- [ ] **Phase 4.4 — Verify toolchain** (`cargo check` clean)
- [ ] **Phase 4.5 — Run setup checklist** on local machine (~50 min)
- [ ] Phase 4.6 — Build `keeperhub-rs` MCP client (real list_workflows)
- [ ] Phase 4.7 — Build Aave V3 yield workflow in KeeperHub visual builder
- [ ] Phase 4.8 — Build Morpho health-check workflow
- [ ] Phase 4.9 — Agent loop skeleton (logs every 60s)
- [ ] Phase 5 — Build phase (Jul 27 → Aug 13)
- [ ] Phase 6 — Post-submit

## Read first

1. **[plans/moltbot-deep-research.md](plans/moltbot-deep-research.md)** — the full research document.
2. **[plans/keeperhub-docs-summary.md](plans/keeperhub-docs-summary.md)** — quick-reference card for the KeeperHub platform.
3. **[plans/setup-verified.md](plans/setup-verified.md)** — the local environment checklist (~50 min).

## Quickstart

```sh
# Verify the toolchain and scaffold compile
cd ~/dev/moltbot
cargo check

# Run the binary (placeholder for now)
cargo run -p moltbot

# Run the example
export KEEPERHUB_API_KEY=kh_your_key_here
cargo run -p keeperhub-rs --example list_workflows

# Lint
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

## TL;DR

Every other submission will be a KeeperHub *workflow* or a KeeperHub-*backed* agent. MoltBot is a KeeperHub *customer* — the first paying customer in the keeper economy. Different layer, different story, harder to fake.

The build also produces a `keeperhub-rs` Rust client that fills the only stack gap in KeeperHub's ecosystem (they ship TS + Python adapters; no Rust). That crate stacks with the grand prize as the $1k onboarding bounty deliverable.
