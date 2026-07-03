# MoltBot

> First paying customer of the KeeperHub marketplace. A Rust autonomous agent that funds itself by paying onchain keepers via x402, earning yield on idle USDC, and auditing every economic decision through KeeperHub's audit trail.

**Repo:** [github.com/AduAkorful/moltbot](https://github.com/AduAkorful/moltbot)
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
├── plans/
│   ├── moltbot-deep-research.md           ← 15 sections, 803 lines
│   ├── next-steps.md                      ← 31 GitHub-ready iterations
│   ├── keeperhub-docs-summary.md          ← quick reference card
│   └── setup-verified.md                  ← local environment checklist
└── scripts/
    ├── README.md
    └── create-issues.sh                   ← bootstrap script for the 31 issues
```

## Where we are

- [x] Phase 1 — Context & constraints (hackathon analyzed)
- [x] Phase 2 — Ideation & scoring (MoltBot chosen, 3 directions compared)
- [x] Phase 3 — Deep product research (15 sections)
- [x] Phase 4.1 — KeeperHub docs summarized (`plans/keeperhub-docs-summary.md`)
- [x] Phase 4.2 — Setup checklist written (`plans/setup-verified.md`)
- [x] Phase 4.3 — Rust workspace scaffolded
- [x] Phase 4.4 — Toolchain verified (Rust 1.96.1, cargo on PATH)
- [x] Phase 4.5 — Iterations plan written (`plans/next-steps.md`, 31 issues)
- [x] **Phase 4.6 — GitHub issues created** (31 issues, 19 labels, 4 milestones)
- [ ] **Phase 4.7 — Run local setup checklist** (~50 min, blocks all code work — [issue #1](https://github.com/AduAkorful/moltbot/issues/1))
- [ ] Phase 5 — Build (Jul 27 → Aug 13, see [issues #16-24](https://github.com/AduAkorful/moltbot/issues?q=is%3Aissue+milestone%3A%22Phase+5%3A+Build+%2B+Demo%22))
- [ ] Phase 6 — Post-submit ([issues #29-31](https://github.com/AduAkorful/moltbot/issues?q=is%3Aissue+milestone%3A%22Phase+6%3A+Post-submit%22))

## Read first

1. **[plans/moltbot-deep-research.md](plans/moltbot-deep-research.md)** — the full research document. Win strategy, competitive landscape, architecture.
2. **[plans/next-steps.md](plans/next-steps.md)** — **the iteration plan. 31 GitHub-ready issues with acceptance criteria, dependency chains, and a week-by-week schedule. Read this to know what's next.**
3. **[plans/keeperhub-docs-summary.md](plans/keeperhub-docs-summary.md)** — quick-reference card for the KeeperHub platform.
4. **[plans/setup-verified.md](plans/setup-verified.md)** — the local environment checklist (~50 min).

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

## Iteration tracking

**31 issues live on GitHub** — see the [Issues tab](https://github.com/AduAkorful/moltbot/issues) for the full backlog.

- 21 P0 (blocks grand prize)
- 8 P1 (enhances the demo)
- 2 P2 (ship if time)
- 15 in pre-build milestone (Jul 27 deadline)
- 9 in build + demo milestone (Aug 13 deadline)
- 4 in bounty milestone (parallel with build)
- 3 in post-submit milestone (Aug 20)

**The next unblocked issue is [#1 — Run local setup checklist](https://github.com/AduAkorful/moltbot/issues/1)** (~50 min, blocks all code work).

The full plan is in [`plans/next-steps.md`](plans/next-steps.md) — issue cards include the same content as the GitHub issues, plus dependency chains, week-by-week schedule, and a "what we are NOT building" list.
