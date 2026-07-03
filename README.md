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
│   ├── moltbot-deep-research.md           ← strategy, competitive landscape, architecture
│   ├── next-steps.md                      ← 31-iteration build plan (the working doc)
│   ├── keeperhub-docs-summary.md          ← quick reference card
│   └── setup-verified.md                  ← local environment checklist
└── scripts/
    ├── README.md
    └── create-issues.sh                   ← (legacy) bootstrap script, no longer used
```

## Where we are

- [x] Phase 1 — Context & constraints (hackathon analyzed)
- [x] Phase 2 — Ideation & scoring (MoltBot chosen, 3 directions compared)
- [x] Phase 3 — Deep product research (15 sections)
- [x] Phase 4.1 — KeeperHub docs summarized (`plans/keeperhub-docs-summary.md`)
- [x] Phase 4.2 — Setup checklist written (`plans/setup-verified.md`)
- [x] Phase 4.3 — Rust workspace scaffolded
- [x] Phase 4.4 — Toolchain verified (Rust 1.96.1, cargo on PATH)
- [x] Phase 4.5 — Iterations plan written (`plans/next-steps.md`, 31 iterations)
- [ ] **Phase 4.6 — Run local setup checklist** (~50 min, blocks all code work — see "Next" below)
- [ ] Phase 5 — Build (Jul 27 → Aug 13, see `plans/next-steps.md` weeks 4-6)
- [ ] Phase 6 — Post-submit

## Read first

1. **[plans/moltbot-deep-research.md](plans/moltbot-deep-research.md)** — the full research document. Win strategy, competitive landscape, architecture.
2. **[plans/next-steps.md](plans/next-steps.md)** — **the working doc. 31 iterations with acceptance criteria, dependency chains, and a week-by-week schedule. Read this to know what's next.**
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

We use **`plans/next-steps.md` as the single source of truth** — no GitHub Issues, no project board. The doc has 31 iteration cards, each with:

- **Why** (one-sentence justification)
- **Depends on** (issue numbers as `##N` references that resolve in the doc)
- **What to do** (concrete steps)
- **Acceptance criteria** (checkboxes)
- **Done =** (one-line definition)
- **Priority** (P0 / P1 / P2), **Estimate** (S / M / L / XL), **Track** (A: keeperhub-rs, B: KeeperHub workflows, C: moltbot agent, D: dashboard/demo)

**The next unblocked iteration is #1 — Run local setup checklist** (~50 min, blocks all code work). Open `plans/next-steps.md`, find the "Issue cards" section, and start at the top.

When you complete an iteration, edit the doc to check the acceptance criteria, add a commit, and move on.
