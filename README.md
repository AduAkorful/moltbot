# MoltBot

> First paying customer of the KeeperHub marketplace. A Rust autonomous agent that funds itself by paying onchain keepers via x402, earning yield on idle USDC, and auditing every economic decision through KeeperHub's audit trail.

**Competition:** [KeeperHub — Agents Onchain Hackathon](https://dorahacks.io/hackathon/agents-onchain/detail) (DoraHacks)
**Build window:** Jul 27 → Aug 13, 2026
**Pre-build starts:** Jul 3, 2026

## Project layout

```
moltbot/
├── README.md           ← you are here
├── .gitignore
├── crates/             ← Rust workspace (keeperhub-rs + moltbot binary)
└── plans/              ← planning docs, research, design decisions
    └── moltbot-deep-research.md
```

## Where we are

- [x] Phase 1 — Context & constraints (hackathon analyzed)
- [x] Phase 2 — Ideation & scoring (MoltBot chosen, 3 directions compared)
- [x] Phase 3 — Deep product research (15 sections, 6k words)
- [ ] Phase 4 — Pre-build (now → Jul 27): `keeperhub-rs` skeleton, MCP client, x402 client, yield workflow, agent loop scaffold
- [ ] Phase 5 — Build (Jul 27 → Aug 13): integration, demo video, submission
- [ ] Phase 6 — Post-submit: bounty deliverable, marketing, crates.io publish

## Read first

1. **[plans/moltbot-deep-research.md](plans/moltbot-deep-research.md)** — the full research document. Win strategy, competitive landscape, architecture, 17-day build plan, risk register, submission copy.

## TL;DR

Every other submission will be a KeeperHub *workflow* or a KeeperHub-*backed* agent. MoltBot is a KeeperHub *customer* — the first paying customer in the keeper economy. Different layer, different story, harder to fake.

The build also produces a `keeperhub-rs` Rust client that fills the only stack gap in KeeperHub's ecosystem (they ship TS + Python adapters; no Rust). That crate stacks with the grand prize as the $1k onboarding bounty deliverable.
