# MoltBot — Deep Project Research

**Competition:** KeeperHub — Agents Onchain Hackathon (DoraHacks)
**Prize target:** Grand Prize ($2,000 1st place)
**Stack constraint:** Rust + Solidity heavy, solo builder
**Build window:** Jul 27 → Aug 13, 2026 (17 days)
**Pre-build window:** 24 days (now → Jul 27)
**Status:** No BUIDLs submitted yet. Clean field.

---

## 0. Executive Summary

**MoltBot is the first autonomous onchain agent that uses KeeperHub's paid workflow marketplace as its economic body.** It holds a USDC wallet, pays KeeperHub keepers via x402 for onchain work (supply to Aave, read prices, claim rewards, swap on Uniswap), earns yield on idle USDC, and logs every decision through KeeperHub's audit trail. It is the *first paying customer* in KeeperHub's marketplace and the first *end-to-end demonstration* of the marketplace flywheel.

**Why it wins the grand prize:**
- The judging rubric asks for "real-world usefulness — would anyone actually run this?" The answer is yes: every agentic SaaS that needs a wallet that pays its own way.
- It uses **every KeeperHub surface** judges reward: MCP server, x402 + MPP, paid workflows, audit trail, smart gas, private routing.
- It is **impossible to fake** — judges must see real onchain transactions, real x402 payments, real audit entries. A polished demo that "never touches a chain" gets disqualified.
- It is **original by combination**, not by invention: [self-driving treasury](https://gov.uniswap.org) + [x402 micropayments](https://www.x402.org/) + [KeeperHub marketplace](https://keeperhub.com) = first *paying customer* in agent-to-keeper commerce.

**The standing-out strategy, in one line:** *Every other submission will be a KeeperHub workflow or a KeeperHub-backed agent. MoltBot is neither — it is an agent whose economy is the KeeperHub marketplace itself.*

**The hidden secondary win (the $1k onboarding bounty):** the Rust client library we build to call KeeperHub from Rust is itself the bounty deliverable. We can ship MoltBot and a `keeperhub-rs` crate (or PR against the existing KeeperHub org) in parallel.

---

## 1. Product Redefinition (after KeeperHub docs research)

The original MoltBot sketch was: "an agent that funds itself via yield + pays for data via x402." After reading the KeeperHub docs in full, the product is **sharper and more original**:

### 1.1 What MoltBot actually is

A Rust agent binary that:
1. Holds a USDC balance on Base (via the KeeperHub agentic wallet, or its own ethers-rs / alloy wallet).
2. Has a *job* (a thing it has been told to do — e.g., "keep my Aave position healthy", "watch ETH price and alert me", "auto-compound my Morpho vault").
3. Pays KeeperHub keeper workflows in USDC via x402 to do work that requires onchain execution (read state, supply, withdraw, swap, claim, transfer).
4. When its USDC balance is below a threshold, parks the rest in Aave V3 via a KeeperHub yield workflow. Earns yield. Draws down only when needed.
5. Optionally *receives* USDC from users who want it to do work for them (the marketplace-reversed pattern: MoltBot is also a KeeperHub workflow, billed per execution).
6. Logs every action through KeeperHub's audit trail. Every onchain tx hash, every x402 payment, every decision.

### 1.2 The combination originality triple

```
[Self-driving treasury / yield-bearing wallet]   ← proven mechanic (Yearn, Aave, Coinbase Earn)
+ [x402 micropayments for per-call services]     ← new context (Coinbase, Cloudflare, Google all building this)
+ [KeeperHub's paid workflow marketplace]        ← stack-unique primitive (no other platform has this flywheel)
= MoltBot (the first paying customer of the keeper economy)
```

### 1.3 The two-layer name

**Molt** — to shed a former state and grow a new one. The agent "molts" out of needing a human to fund it. It earns, it spends, it adapts, it persists.

**Bot** — autonomous, with a wallet, doing economic work.

**MoltBot** = an economic actor that bootstraps its own existence, sheds its dependence on a human payer, and grows into self-sufficiency.

### 1.4 What makes this more than "another yield-wrapped agent"

Most "self-funding agent" demos are:
- Single-strategy (just Aave supply)
- Don't actually pay for anything (the "self-funding" is theoretical)
- No real audit trail
- No external services consumed

MoltBot differs on every axis:
- **Pluggable strategy** — any KeeperHub workflow is a callable "income source"
- **Real spending** — actually pays for keeper work via x402
- **Real audit** — every economic decision is in KeeperHub's `get_execution_logs`
- **Real services** — the agent's *job* is the thing users actually want done

---

## 2. Competitive Landscape (deep audit)

The competitive space has three layers, each with a different incumbent. We need to know all three to position correctly.

### 2.1 Layer 1 — Agent frameworks that could host MoltBot

| Project | Lang | Stars | x402 | MCP | Notes | Threat to MoltBot? |
|---|---|---|---|---|---|---|
| **qntx/machi** | Rust | 564 | ✅ | ✅ | "Agent behavior that compiles." FSL-1.1-ALv2 (no competing use for 2yr). Closest competitor in *our exact stack*. | **Medium** — same stack, similar goals. *Opportunity:* we can use machi as our agent runtime and focus on the *application* layer. Their license actually *prevents* them from shipping a hosted version of a self-funding agent for 2 years. |
| **daydreamsai/lucid-agents** | TypeScript | 187 | ✅ | — | "Bootstrap agents in 60 seconds that can pay, sell, participate in commerce." Protocol-agnostic, Hono/Express/Next adapters. | Low — TS only, different stack. |
| **BlockRunAI/ClawRouter** | TypeScript | 6.6k | ✅ | — | LLM router that pays for inference. High star count. | Low — different product (router, not economic agent). |
| **BlockRunAI/blockrun-mcp** | TypeScript | 468 | ✅ | ✅ | MCP server for pay-per-call data feeds. | Low — they're a *supplier* in the x402 economy, not a consumer. *Opportunity:* MoltBot can buy data from them. |
| **vybenetwork/x402-client** | TypeScript | 169 | ✅ | — | Client SDK for Solana analytics API. | Low — narrow scope. |
| **qntx/x402-openai-python** | Python | 260 | ✅ | — | OpenAI client with transparent x402. | Low — narrow scope. |
| **google-agentic-commerce/a2a-x402** | Python | 531 | ✅ | — | Google's official A2A x402 extension. | Low — different protocol, Google's blessing means it won't move fast. |
| **KeeperHub/eve-plugin** | TypeScript | 0 | via MCP | ✅ | Vercel Eve integration for KeeperHub. | Low — frame adapter. |
| **KeeperHub/hermes-plugin** | Python | 0 | via MCP | ✅ | Hermes framework integration. | Low — frame adapter. |
| **KeeperHub/mcp** | TS + Python | 0 | ✅ | ✅ | "Shared MCP client foundation for KeeperHub agent-framework adapters (TypeScript + Python)." | **Direct callout** — KeeperHub *explicitly lists* TS and Python adapters as supported. **Rust is the gap.** |
| **KeeperHub/sdk** | TypeScript | 0 | ✅ | — | Official REST SDK. TS only. | Direct callout — no Rust SDK. |

**The Rust gap is real.** Every official KeeperHub client is TypeScript or Python. A Rust client/agent is a category-creating contribution to the ecosystem. *This is also the bounty deliverable.*

### 2.2 Layer 2 — "Self-funding" or "autonomous wallet" agents

| Project | Status | Notes |
|---|---|---|
| Coinbase "Based Agent" (2024) | Launched, deprecated | "An AI agent with its own crypto wallet." Hand-wavy on funding model. Press play, no production. |
| aixbt by @0rxbt | Live | Twitter agent that talks about tokens. Wallet is for *receiving tips*, not self-funding. |
| Terminal of Truths (ToT) | Live | AI agent that posts on X. Wallet receives GOAT tips. No real "self-funding loop." |
| Virtuals Protocol agents | Live | Agents with onchain wallets. Most are "monetized influencers," not economic actors. |
| Bankr / Clanker / Clawdbot | Various | Mostly social agents with tipping. |
| Coinbase Developer Platform "Agentic Wallet Skills" | Live | 9 skills (auth, fund, pay, search, send, trade, query, x402). *Tools* for agents to have wallets. Not an agent itself. |

**None of these have a self-funding loop.** They all rely on a human or a treasury to keep the wallet topped up. The closest analog is "a DAO treasury that operates itself" — there's prior art in [Safe Modules](https://docs.safe.global), [Zodiac Roles Modifier](https://github.com/gnosis/zodiac-modifier-roles), and the [Sablier](https://sablier.com) streaming-payroll pattern. But these are *passive* — they execute pre-set rules. MoltBot is *active* — it reasons about what to buy and what to skip.

**MoltBot is the first *active* self-funding agent built on top of a paid workflow marketplace.**

### 2.3 Layer 3 — KeeperHub ecosystem and likely competitors in this hackathon

With 34 hackers registered and a $5k pool, the field will be a mix of:

| Likely submission | What it is | How it scores vs. MoltBot |
|---|---|---|
| A KeeperHub **workflow** built in the visual builder (e.g., "auto-rebalance Aave position", "Morpho claim compounder") | A static, pre-scheduled automation. | Strong on execution (it's a real KeeperHub run). Weak on originality (it's a workflow, like dozens of others). |
| A KeeperHub workflow that uses x402 (e.g., a paid workflow that other agents call) | A supplier in the marketplace. | Medium originality. Good surface coverage. |
| An ElizaOS / CrewAI / LangChain agent that does one thing (e.g., a swap bot, a price oracle, a liquidation watcher) | A framework demo, framework-agnostic. | High surface coverage. Lower originality (framework has been demoed many times). |
| A Solidity smart contract that does something interesting (e.g., a novel vault, a new AMM) | A pure-Solidity entry. | Zero KeeperHub usage = disqualified by the one hard requirement. |
| A nice template / tutorial / docs PR (the bounty path) | A contribution. | Different prize track. |
| **MoltBot** | **A Rust agent that pays for keeper work, earns yield, audits everything.** | **Highest originality. Full surface coverage. Impossible to fake.** |

**The competitive edge: every other submission is a *user* of KeeperHub. MoltBot is a *customer* of KeeperHub.** That distinction is what wins the grand prize.

### 2.4 What nobody in this space has built

- ❌ An agent whose *wallet balance* is the only thing the user has to manage
- ❌ An agent that calls a *paid workflow marketplace* as part of its core loop
- ❌ A Rust-native client for KeeperHub (no SDK, no adapter, no plugin)
- ❌ An end-to-end demo of the KeeperHub marketplace flywheel: agent pays keeper → keeper earns → keeper is incentivized to publish more workflows → marketplace grows
- ❌ A agent that uses KeeperHub's audit trail as its *primary observability surface* (no agent framework today renders KeeperHub's runs panel as a first-class view)

**MoltBot ships all five.**

---

## 3. Win Strategy (rubric-by-rubric)

The judging criteria from the hackathon page, with how MoltBot scores on each sub-criterion.

### 3.1 "Does it execute onchain via KeeperHub?" (heavy weight)
**Target: 5/5.**
- Every MoltBot action goes through `call_workflow` on the KeeperHub MCP server.
- A real onchain tx hash will be linked in the submission.
- Multiple tx types: `web3/transfer-funds`, `web3/write-contract` (Aave supply/withdraw), `web3/transfer-token` (USDC payment).
- The agent runs for at least 24 hours before the submission deadline, generating 20+ real onchain transactions.

### 3.2 "Use of KeeperHub surfaces" (MCP server, CLI, x402, MPP, workflow builder, audit trail)
**Target: 5/5.** The full surface map:

| Surface | How MoltBot uses it | Evidence |
|---|---|---|
| **MCP server** | `call_workflow`, `search_workflows`, `get_execution_logs` are the agent's three primary tools | Code in `src/mcp_client.rs`; logs in submission |
| **x402** | Agent pays for paid keeper workflows in USDC on Base | x402scan entries linked in submission |
| **MPP** | Agent optionally pays via Tempo USDC.e on Tempo chain | Code path included even if mainnet not used |
| **Workflow builder** | Agent *itself* could be published as a workflow, but the *core demo* uses existing marketplace workflows | Screenshots of search results |
| **Audit trail** | Every MoltBot action rendered as a KeeperHub run | Live URL or static dashboard with the agent's run history |
| **Smart gas estimation** | Inherited automatically from KeeperHub's `web3/*` write actions | Mentioned in README, demonstrated in run logs |
| **Private routing / MEV protection** | Inherited automatically from KeeperHub | Mentioned in README, noted in submission copy |
| **Gas sponsorship** | Use KeeperHub's gas sponsorship on Ethereum mainnet (no ETH needed) | Used for at least one execution |
| **CLI** | `kh workflow list` and `kh execute` are part of the demo script | Terminal recording in demo video |

### 3.3 "Reliability and observability"
**Target: 5/5.**
- Retries: inherit from KeeperHub's exponential backoff (no custom code needed).
- Gas handling: same — KeeperHub handles it.
- Audit trail: this is the *primary* observability for the agent. Every tx has trigger, simulation, gas used, outcome, timestamp. The demo shows the runs panel.
- Failure modes: the agent has a `safe-mode` where it switches to a "read-only, alert-only" posture if the USDC balance goes below a hard floor. This is documented and demonstrated.

### 3.4 "Originality and real-world usefulness"
**Target: 5/5.**
- Combination originality triple is novel (no direct equivalent in the wild).
- Real-world use case: every agentic SaaS that needs a wallet that pays its own way. Imagine: a customer-support agent that pays for its own LLM calls. A data-scraping agent that pays for proxies. A DeFi agent that pays for its own keeper work. The substrate is reusable.
- The "killer demo" — a 60-second video where a human gives MoltBot a job, walks away, and the agent earns, spends, and reports back — is the kind of demo judges remember.
- A second-order originality: the Rust client crate that emerges is a contribution to the KeeperHub ecosystem. This is a *category-creating* submission, not a feature add.

### 3.5 "Integration quality and developer experience"
**Target: 5/5.**
- Clean Rust code, well-commented, well-tested.
- A `keeperhub-rs` crate published to crates.io as a byproduct (also helps the bounty).
- README with diagrams of the agent loop, a 5-minute quickstart, and a 1-command `cargo run --example moltbot-demo` to reproduce the demo.
- A "what I would build next" section in the README that signals long-term thinking.
- A merged PR or open issue to the KeeperHub org documenting the Rust integration as a follow-up.

### 3.6 Anti-pattern audit (verifying we are not in any of the named traps)

| Anti-pattern | Status |
|---|---|
| **The Clone** — building something that exists well elsewhere | ❌ No. Closest analogs (qntx/machi, lucid-agents) don't have a self-funding loop tied to a paid marketplace. |
| **The Overscope** — too ambitious to ship in 17 days | ⚠️ Borderline. The *core* loop is 3 components (MCP client + x402 client + a yield workflow). The 17-day plan is tight but achievable for a focused solo dev. |
| **The Underfit** — simple but misaligned with scoring | ❌ No. Every scoring dimension is explicitly addressed. |
| **The Single-Pillar** — great at one thing, mediocre elsewhere | ❌ No. Surface coverage is intentionally broad. |
| **The No-Audience** — technically interesting but no user | ❌ No. Target audience is every team building an agentic SaaS that needs an autonomous wallet. |

---

## 4. Product Definition (Phase 3 §1, concrete)

```
MoltBot is a Rust autonomous economic agent that uses KeeperHub's paid
workflow marketplace as its execution and earning layer. The agent holds
a USDC balance on Base, parks idle funds in Aave V3 via a KeeperHub
yield workflow, and pays for keeper work in real time via x402 whenever
it needs to read state, claim rewards, or move funds. Every economic
decision is logged through KeeperHub's audit trail, making the agent's
full economic life replayable from a single page. The name has two
layers: "molt" — to shed a former state and grow a new one — and "bot"
— an autonomous actor. MoltBot sheds its dependence on a human payer
and grows into an economic actor that funds its own existence.
```

The MoltBot is **not** a KeeperHub workflow. It is a Rust binary that *calls* KeeperHub. The distinction is the entire product.

---

## 5. Core Mechanic Decisions (Phase 3 §4)

### 5.1 The agent loop

Every N seconds (configurable, default 60s), MoltBot:
1. Reads its USDC balance via `web3/check-token-balance`.
2. Computes a target posture: e.g., "if balance > $50, park 80% in Aave; if balance < $5, withdraw from Aave."
3. If the posture requires a state change, calls the appropriate KeeperHub workflow via `call_workflow` (e.g., `aave-supply-usdc` or `aave-withdraw-usdc`).
4. Then runs its *job* — e.g., "monitor the Morpho vault for health factor < 1.2 and supply more collateral if so." This requires a *paid* workflow like `morpho-check-position` and possibly `aave-supply`.
5. Pays for each paid workflow via x402.
6. Logs every action via the KeeperHub MCP audit trail.
7. Repeats.

**Decision: Loop frequency 60s.** Faster is unnecessary; slower misses events. 60s also gives the audit trail a visible cadence.

### 5.2 The yield strategy

**Options considered:**

| Option | Pros | Cons |
|---|---|---|
| Aave V3 supply | Highest TVL, most liquid, well-supported by KeeperHub plugin, well-known | Yield varies, gas can spike on L1 (we use Base) |
| Morpho supply | Potentially higher yield, blue-chip vaults (steakhouse, MEV Capital) | Less liquid exit, more complex |
| Yearn V3 vault | Auto-compounding | Less direct, adds dependency |
| Spark / Sky | Highest yield on stablecoins currently | Newer, less battle-tested |
| **Aave V3 (chosen)** | Liquidity, simplicity, well-supported, known risk profile | — |

**Decision: Aave V3 on Base.** Reasons: best supported by KeeperHub (Aave V3 plugin exists), deepest liquidity for instant exit, most users have heard of it (judges won't need explanation), and Base means cheap transactions.

**Decision: park 80% of balance above $50, withdraw when below $20.** Reasons: 20% buffer for paying x402 without round-tripping yield constantly, $20 floor is the "safe mode" trigger.

### 5.3 The job (the user's actual ask)

MoltBot needs *a* job for the demo. The most demonstrable one: **"Keep my Morpho ETH-collateralized position healthy. If health factor drops below 1.3, supply more wstETH from a pre-funded balance. If the position earns > $5 in rewards, claim and re-deposit."**

Why this job:
- Multi-step (read state → decide → execute)
- Has clear economic value (avoids liquidation)
- Uses several KeeperHub plugins (Morpho, Aave for the wstETH borrow, possibly Chainlink for the price feed)
- The "decision" is auditable (you can see the threshold check in the audit trail)

**Decision: ship with one hard-coded job, but design the job runner to be pluggable** — a `Job` trait in Rust, with one concrete implementation for the demo. A second example job (e.g., "watch ETH price and alert via Telegram if it drops 5% in an hour") ships as a separate example file.

### 5.4 The wallet

Two options:

| Option | Pros | Cons | Suitability |
|---|---|---|---|
| **KeeperHub agentic wallet** | Free, no private key on disk, x402 auto-pay built-in, Turnkey-secured | $200/day cap, $100/tx cap, only Base USDC + Tempo USDC.e | **Demo-friendly**, but the cap limits the "earn real yield" story |
| **Self-custodied Rust wallet** (alloy-rs) | Full control, no caps, can do anything | Must implement x402 client ourselves, must handle gas sponsorship manually | More impressive technically, but riskier for 17 days |

**Decision: start with the KeeperHub agentic wallet for the demo, with a clear path to self-custody.** The agentic wallet proves the *integration*. The cap is fine for the demo ($5 of yield pays for 100 x402 calls). The self-custody path is documented in the README and partially implemented in a `wallet/self_custody` feature flag — this also doubles as the bounty deliverable's foundation.

### 5.5 Failure modes

| Failure | Detection | Response |
|---|---|---|
| USDC balance below $5 | Loop step 1 | **Safe mode**: stop all job execution, send Telegram alert via `telegram` plugin, log to audit trail |
| Aave withdraw reverts | Loop step 3 | Retry with backoff (inherits from KeeperHub); after 3 fails, skip cycle and alert |
| x402 402 with ask-tier amount | `get_wallet_integration` returns 402 with amount > $5 | Skip the call, log it, move on (we don't want to ask the user) |
| KeeperHub MCP server down | `call_workflow` returns 5xx | Backoff exponentially up to 5 minutes, then alert |
| Onchain reorg undoes an Aave supply | Read state on next loop, detect discrepancy, re-execute | Handled naturally by the next loop iteration |

---

## 6. Information Architecture & Data Model (Phase 3 §9)

### 6.1 Entities

```
Entity: AgentConfig
  Fields:
    wallet_address: String — KeeperHub agentic wallet address
    rpc_url: String — Base mainnet RPC (Alchemy/Infura)
    loop_interval_seconds: u32 — default 60
    yield_strategy: Enum<Aave, Morpho, Spark> — default Aave
    yield_asset: Enum<USDC, USDC.e> — default USDC
    park_threshold_usdc: f64 — default 50.0
    withdraw_threshold_usdc: f64 — default 20.0
    safe_mode_threshold_usdc: f64 — default 5.0
    max_x402_payment_usd: f64 — default 0.10
  Primary key: wallet_address
  Where it lives: TOML file in repo + .env for secrets

Entity: RunRecord (one per agent loop iteration)
  Fields:
    iteration: u64
    started_at: DateTime
    ended_at: DateTime
    usdc_balance_before: f64
    usdc_balance_after: f64
    actions_taken: Vec<Action>
    keeper_runs: Vec<String> — KeeperHub execution IDs
    onchain_txs: Vec<String> — tx hashes
    x402_payments: Vec<X402Payment>
    safe_mode: bool
    error: Option<String>
  Primary key: iteration
  Where it lives: SQLite local DB (for the demo dashboard) + KeeperHub audit trail (for judges)

Entity: Action
  Fields:
    kind: Enum<YieldPark, YieldWithdraw, JobRead, JobExecute, Alert>
    keeper_workflow_slug: String
    inputs: serde_json::Value
    result: serde_json::Value
    tx_hash: Option<String>
    x402_payment_usd: Option<f64>
    timestamp: DateTime
    status: Enum<Pending, Success, Failed, Skipped>

Entity: X402Payment
  Fields:
    amount_usd: f64
    asset: Enum<USDC, USDC.e>
    chain: Enum<Base, Tempo>
    facilitator: String
    tx_hash: String
    timestamp: DateTime

Entity: Job
  Fields:
    name: String
    config: serde_json::Value
    job_trait: String — Rust module path
    enabled: bool
```

### 6.2 Storage decisions

| Data | Where | Reasoning |
|---|---|---|
| AgentConfig | TOML in repo | Static after build |
| RunRecord | SQLite (local) + KeeperHub audit trail | Local for the demo dashboard; on KeeperHub for judges' inspection |
| Action, X402Payment | Embedded in RunRecord | Subordinate to the parent record |
| Wallet secrets | `.env` (gitignored) + KeeperHub's `~/.keeperhub/wallet.json` for agentic wallet | Standard |
| KeeperHub MCP tokens | `.env` | Standard |
| Onchain tx hashes | KeeperHub's audit trail (primary) + SQLite (cache) | Source of truth is KeeperHub |

### 6.3 Does this need a backend?

**Yes, but minimal.** The demo dashboard needs:
- A simple HTTP server to serve the audit trail view (Axum, ~200 LOC)
- A SQLite database (sqlx)
- Optional: a static export (for judges who don't want to run the binary)

**No backend required for:**
- The agent itself (it runs locally on the developer's machine)
- The KeeperHub integration (KeeperHub is the backend)
- x402 payments (settle on Base / Tempo)

---

## 7. Complete Architecture (file tree + flow)

```
moltbot/
├── Cargo.toml                     # workspace root
├── README.md                      # main entry
├── ARCHITECTURE.md                # this doc, exported
├── DEMO.md                        # how to run the demo
├── moltbot.toml.example           # sample config
├── crates/
│   ├── keeperhub-rs/              # the bounty deliverable: standalone KeeperHub Rust client
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs             # public API
│   │   │   ├── mcp.rs             # MCP server client (over HTTP/SSE)
│   │   │   ├── rest.rs            # REST API client (workflows, executions, wallets)
│   │   │   ├── x402.rs            # x402 auto-pay client (EIP-3009 TransferWithAuthorization)
│   │   │   ├── mpp.rs             # MPP protocol support (optional, feature-flagged)
│   │   │   ├── types.rs           # shared types (Workflow, Execution, Run, etc.)
│   │   │   └── error.rs           # error types
│   │   ├── examples/
│   │   │   └── list_workflows.rs
│   │   └── tests/
│   │       ├── mcp.rs
│   │       └── x402.rs
│   │
│   └── moltbot/                   # the agent binary
│       ├── Cargo.toml             # depends on keeperhub-rs
│       ├── src/
│       │   ├── main.rs            # entry, arg parsing, top-level loop
│       │   ├── config.rs          # AgentConfig loading
│       │   ├── loop.rs            # the main agent loop
│       │   ├── yield_strategy.rs  # Aave V3 yield plug-in
│       │   ├── job.rs             # Job trait + dispatcher
│       │   ├── jobs/
│       │   │   ├── mod.rs
│       │   │   ├── morpho_health.rs   # the demo job
│       │   │   └── price_alert.rs     # second example job
│       │   ├── safe_mode.rs       # safe-mode detection + alert
│       │   ├── audit.rs           # SQLite + audit trail writer
│       │   └── dashboard.rs       # Axum server, simple HTML/JS UI
│       ├── migrations/
│       │   └── 001_init.sql
│       ├── static/
│       │   ├── index.html         # the audit trail dashboard
│       │   ├── app.js
│       │   └── style.css
│       └── tests/
│           ├── loop.rs
│           └── yield_strategy.rs
│
└── docs/
    ├── architecture.png           # diagram of the agent loop
    ├── demo-script.md             # the demo video script
    └── postmortem.md              # written after submission
```

### 7.1 The flow at a glance

```
                        ┌─────────────────────────────────┐
                        │         MoltBot (Rust)          │
                        │                                 │
   every 60s:           │  ┌─────────────────────────┐    │
   ┌───────────────────►│  │   agent loop            │    │
   │                    │  │                         │    │
   │                    │  │  1. read USDC balance   │    │
   │                    │  │  2. decide posture      │    │
   │                    │  │  3. maybe yield action  │    │
   │                    │  │  4. run job             │    │
   │                    │  │  5. log everything      │    │
   │                    │  └────────┬────────────────┘    │
   │                    └───────────┼─────────────────────┘
   │                                │
   │                  keeperhub-rs  │  (HTTP + JSON-RPC)
   │                                ▼
   │              ┌──────────────────────────────────────┐
   │              │      KeeperHub MCP Server            │
   │              │                                      │
   │              │  • search_workflows                  │
   │              │  • call_workflow(slug, inputs)       │
   │              │  • get_execution_status              │
   │              │  • get_execution_logs                │
   │              └────────┬─────────────────────────────┘
   │                       │
   │       ┌───────────────┼───────────────────┐
   │       ▼               ▼                   ▼
   │  ┌─────────┐    ┌──────────┐      ┌─────────────┐
   │  │  Aave   │    │  Morpho  │      │  Chainlink  │
   │  │ workflow│    │ workflow │      │  workflow   │
   │  │ (yield) │    │  (job)   │      │   (job)     │
   │  └────┬────┘    └─────┬────┘      └──────┬──────┘
   │       │               │                  │
   │       └───────────────┼──────────────────┘
   │                       │
   │                       ▼
   │              ┌──────────────────┐
   │              │  Base / Tempo    │
   │              │  (onchain txs)   │
   │              └────────┬─────────┘
   │                       │
   │                       ▼
   │              ┌──────────────────────────┐
   └──────────────┤  x402 settlement (USDC)  │
                  │  via facilitator         │
                  └──────────────────────────┘
```

### 7.2 The x402 payment flow in detail

```
MoltBot            keeperhub-rs         KeeperHub MCP         Facilitator          Base chain
  │                      │                      │                    │                    │
  │ call_workflow(s)     │                      │                    │                    │
  │─────────────────────►│                      │                    │                    │
  │                      │ POST /mcp call       │                    │                    │
  │                      │─────────────────────►│                    │                    │
  │                      │                      │ (paid workflow)    │                    │
  │                      │                      │ return 402 + price │                    │
  │                      │◄─────────────────────│                    │                    │
  │                      │                      │                    │                    │
  │                      │ x402 client:         │                    │                    │
  │                      │   build EIP-3009     │                    │                    │
  │                      │   TransferWithAuth   │                    │                    │
  │                      │   for $0.05 USDC     │                    │                    │
  │                      │                      │                    │                    │
  │                      │ sign via Turnkey     │                    │                    │
  │                      │   (KeeperHub proxy)  │                    │                    │
  │                      │                      │                    │                    │
  │                      │ retry with X-PAYMENT │                    │                    │
  │                      │─────────────────────►│                    │                    │
  │                      │                      │ verify payment     │                    │
  │                      │                      │───────────────────►│                    │
  │                      │                      │                    │ settle on Base     │
  │                      │                      │                    │───────────────────►│
  │                      │                      │                    │ tx confirmed      │
  │                      │                      │◄───────────────────│                    │
  │                      │                      │ execute workflow   │                    │
  │                      │                      │   (calls Aave/Morpho onchain)            │
  │                      │                      │───────────────────────────────────────►│
  │                      │                      │                    │  tx confirmed      │
  │                      │◄─────────────────────│                    │                    │
  │ Result { txHash, ..} │                      │                    │                    │
  │◄─────────────────────│                      │                    │                    │
```

---

## 8. Pillar-by-Pillar Win Strategy (Phase 3 §2, specific)

This is the rubric mapped to specific, verifiable behavior in MoltBot.

### 8.1 "Does it execute onchain via KeeperHub?" (target 5/5)

| Sub-criterion | How MoltBot scores 5/5 | Evidence in submission |
|---|---|---|
| **Real transactions, not mockups** | Every loop iteration produces ≥ 1 real onchain tx (Aave supply, Aave withdraw, or x402 settlement) | Run logs in dashboard; 20+ tx hashes in the submission text |
| **Multiple tx types** | `web3/transfer-token` (USDC x402 payment), `web3/write-contract` (Aave supply/withdraw), `web3/transfer-funds` (no — not used) | Listed in README's "Transactions" section |
| **Transaction linked in submission** | The submission's required `tx link` field points to the agent's most recent successful run | Direct Etherscan link in the submission |
| **Happens repeatedly, not once** | Agent runs for ≥ 24 hours, generating a continuous feed of transactions | Dashboard URL is live; static archive in repo |

### 8.2 "Use of KeeperHub surfaces" (target 5/5)

| Surface | Used? | How |
|---|---|---|
| MCP server | ✅ | `call_workflow`, `search_workflows`, `get_execution_logs` in the agent loop |
| CLI | ✅ | Used in the demo video to show `kh workflow list` and `kh execute` |
| x402 | ✅ | Pays for at least one paid workflow per loop iteration (or skips if balance too low) |
| MPP | ✅ | Code path included, feature-flagged, mentioned in README even if mainnet not used |
| Workflow builder | ❌ | The agent itself is *not* a workflow. We mention this explicitly. |
| Audit trail | ✅ | Primary observability for the agent; rendered as the dashboard |
| Smart gas estimation | ✅ | Inherited from `web3/*` write actions; visible in run logs |
| Private routing | ✅ | Inherited; mentioned in README |
| Gas sponsorship | ✅ | Used for at least one execution on Ethereum mainnet (e.g., a small `web3/transfer-funds`) |

**Surface coverage: 8/9 (excluding workflow builder, which by design does not apply to an agent that calls workflows).** This is the broadest possible coverage of relevant surfaces.

### 8.3 "Reliability and observability" (target 5/5)

| Sub-criterion | How MoltBot scores 5/5 |
|---|---|
| **Retries** | Inherited from KeeperHub (exponential backoff on every `web3/*` call). The agent loop also retries the entire iteration on transient MCP errors. |
| **Gas handling** | Inherited from KeeperHub (smart gas estimation + 30% savings vs baseline). The dashboard shows the gas used per tx. |
| **Audit trail usage** | The audit trail is the agent's *primary* observability surface. The dashboard is a thin renderer over `get_execution_logs`. |
| **Failure modes demonstrated** | The submission includes a recorded run where the agent entered safe mode (e.g., USDC balance below $5 after a withdraw) and recovered. |
| **Simulation before submit** | Inherited from KeeperHub's pre-simulation. Mentioned in the demo video. |

### 8.4 "Originality and real-world usefulness" (target 5/5)

| Sub-criterion | How MoltBot scores 5/5 |
|---|---|
| **Solves a real problem** | "Every agent needs a wallet. Most agents need a wallet that pays its own way." This is a real problem for the entire agentic SaaS industry. |
| **Combination originality** | The triple is novel: [self-driving treasury] + [x402 micropayments] + [KeeperHub marketplace] |
| **Non-crypto user can understand** | "An AI that earns its own money and pays for its own work." One sentence. |
| **Target audience clear** | Every team building agentic SaaS that needs an autonomous wallet. Every DAO treasury that wants to operate itself. |
| **At least one person would pay** | Every agent builder. Specifically: dev teams shipping paid agent products today. |

### 8.5 "Integration quality and developer experience" (target 5/5)

| Sub-criterion | How MoltBot scores 5/5 |
|---|---|
| **Cleanly built** | Rust idioms, `cargo clippy --all-targets --all-features -- -D warnings` passes, `cargo fmt` clean |
| **Well-tested** | Unit tests on `keeperhub-rs` (the crate) covering MCP serialization, x402 signing, error paths. Integration tests on the agent loop (mocked MCP). |
| **Well-documented** | README, ARCHITECTURE, DEMO, and inline `///` docs on every public item. |
| **One-command reproduction** | `git clone && cargo run --release --example moltbot-demo` (or close) reproduces the 60-second demo |
| **Long-term thinking** | README has a "Roadmap" section listing next features (self-custody, multi-chain, more yield strategies, agent-to-agent payments) |

---

## 9. Risk Register (Phase 3 §11)

| # | Risk | Probability | Impact | Mitigation | Fallback |
|---|---|---|---|---|---|
| 1 | **x402 402 ask-tier trips a hook prompt** (MCP wants me to approve each payment manually) | Med | High (kills autonomous demo) | Use `auto_approve_max_usd: 0.10` and `block_threshold_usd: 100` so all payments are auto-approved | Skip the paid workflow; document the limitation; rely on free workflows for the demo |
| 2 | **Aave V3 supply workflow costs more in x402 than the yield earns** | Low | Med (the agent loses money) | Pre-compute the breakeven; only park if the expected yield > 5× the x402 cost | Use a different yield strategy (Spark, Morpho) or a static USDC reserve |
| 3 | **MCP server rate-limits the agent** | Low | Med | Implement exponential backoff in the loop | Reduce loop frequency to 5 minutes |
| 4 | **KeeperHub agentic wallet $200/day cap** | Med | Med (caps the demo) | Demo runs for 24 hours, spends ~$2, far below the cap | Document the cap; provide self-custody code path as a follow-up |
| 5 | **The 17-day build slips because of x402 debugging** | High | High | Front-load the x402 client in pre-build (now → Jul 27). It's the riskiest unknown. | Use the KeeperHub agentic wallet's auto-pay *as the only x402 implementation* and remove custom code |
| 6 | **No "yield" KeeperHub workflow exists in the marketplace** at submission time | Med | Med (we have to build one) | Build the Aave V3 yield workflow ourselves (in the KeeperHub visual builder or via the MCP `create_workflow` tool) before we run the demo | Fall back to a swap workflow (Aave supply is a deposit, but a swap to aUSDC on Uniswap would also earn yield) |
| 7 | **A competitor ships a similar agent first** | Low | High | Lean into the Rust advantage; the KeeperHub docs explicitly list TS+Python as the supported adapters — the Rust angle is a moat | Position as "the Rust reference implementation" |
| 8 | **The demo video doesn't land** (judges don't feel the "first paying customer" story) | Med | High | Storyboard the video before recording. Show: (1) human gives MoltBot a job, (2) walks away, (3) dashboard shows the agent earning, paying, working. | Replace video with a long-form README + screenshots + live demo URL |
| 9 | **The Rust MCP client has a subtle bug that causes silent failure** | Med | High | Write integration tests *first* against a live MCP server. Test the failure paths explicitly. | Use the existing TS MCP client as a black box — call it from Rust via a Node subprocess (ugly but works) |
| 10 | **I run out of testnet USDC for the x402 demo** | Low | Low | KeeperHub sponsored gas on Ethereum mainnet; on Base, USDC is cheap; use the Base Sepolia faucet | Use the Base Sepolia testnet (KeeperHub supports it) |

---

## 10. Execution Timeline (Phase 3 §13)

### Pre-build (Jul 3 → Jul 27) — 24 days

This is the *biggest edge*. Use it to eliminate unknowns *before* the 17-day clock starts.

| Days | Goal | Deliverable |
|---|---|---|
| D1–3 | Read all KeeperHub docs end-to-end. Build a mental model of every surface. | Notes file in `docs/keeperhub-research.md` |
| D4–6 | Spin up the KeeperHub agentic wallet. Install the MCP server. Run every example. Verify the surfaces work as documented. | `docs/setup-verified.md` with screenshots of MCP `list_workflows`, an x402 payment, and a KeeperHub run |
| D7–10 | Build the `keeperhub-rs` skeleton. Just the MCP client over HTTP. `list_workflows` and `call_workflow` working. | `crates/keeperhub-rs/src/mcp.rs` with passing tests against the live MCP server |
| D11–13 | Add x402 client to `keeperhub-rs`. Verify auto-pay with a tiny test (pay $0.01 for a real paid workflow). | Working x402 client + test |
| D14–16 | Build the Aave V3 yield KeeperHub workflow in the visual builder. Test it manually. | Workflow ID + run history |
| D17–20 | Build the Morpho health-check KeeperHub workflow. Test it. | Workflow ID + run history |
| D21–24 | Build the agent loop skeleton. `cargo run` does nothing but logs "iteration N started" every 60s. | Working binary |
| D25–26 | **Buffer / catch-up / rest** |  |
| D27 | **Hackathon build phase opens** |  |

**The 17-day clock now starts with: working Rust client, working yield workflow, working job workflow, working agent skeleton.** Only the integration + polish + demo remains.

### Build (Jul 27 → Aug 13) — 17 days

| Days | Theme | Must ship | Marketing | Defer to NEVER |
|---|---|---|---|---|
| D1–2 | Wire the loop | Aave yield action in the agent loop. First onchain tx via MoltBot. | Post the first tx on X with "first MoltBot run" | Multi-chain support |
| D3–4 | Wire the job | Morpho health-check job. Agent makes a real decision based on health factor. | Devlog post: "MoltBot decided to rebalance" | LLM-driven decision-making |
| D5–6 | Audit + dashboard | SQLite persistence + Axum dashboard showing live runs. | Tweet the dashboard URL | Historical replay UI |
| D7 | x402 polish | Make sure every paid call actually pays. Tune auto-approve thresholds. | — | MPP mainnet (feature flag only) |
| D8 | Safe mode | Implement and test safe mode. Force a safe-mode event for the demo. | — | — |
| D9–10 | **Stress test** | Run MoltBot for 24+ hours. Verify 20+ onchain txs. Find and fix any reliability bugs. | Devlog: "MoltBot survived 24h" | More yield strategies |
| D11 | Demo video | Record the 90-second demo video. Edit. | YouTube unlisted, link in submission | Cinematic version |
| D12 | README + submission copy | Write the README, DEMO.md, ARCHITECTURE.md. Draft submission copy. | — | — |
| D13 | Submit EARLY | Submit on DoraHacks. | — | — |
| D14 | **Buffer** | Polish, fix, last touches. | — | — |
| D15 | Bounty deliverable | Push the `keeperhub-rs` crate to crates.io. Open PR to KeeperHub org with a "Rust SDK" proposal. | Post on KeeperHub Discord | — |
| D16 | Post-submission marketing | Post demo everywhere: X, /r/ethdev, /r/solidity, Hacker News, KeeperHub Discord, MoltBot blog. | All channels | — |
| D17 | **Rest. Judging starts Aug 13.** | — | — | — |

### What we are NOT building (named explicitly to prevent scope creep)

- ❌ LLM-driven decision-making (use a hard-coded threshold)
- ❌ Multi-chain support (Base only)
- ❌ Multiple yield strategies (Aave V3 on Base only)
- ❌ Multiple jobs (one hard-coded Morpho job + one example)
- ❌ Self-custody wallet path (mention in README, don't ship)
- ❌ Real-time dashboard updates (use 10s polling)
- ❌ Mobile UI (desktop-only is fine)
- ❌ User authentication (local binary, no auth needed)
- ❌ Multiple concurrent MoltBots (one instance is the demo)

---

## 11. Marketing & Launch Playbook (Phase 3 §12)

### 11.1 The 5-second pitch

> "MoltBot is the first paying customer of the KeeperHub marketplace. It's a Rust agent that earns yield on its USDC, pays for keeper work via x402, and audits every decision. Imagine an AI agent that funds its own existence — that's MoltBot."

### 11.2 The 60-second pitch (for the demo video)

> "Every AI agent needs a wallet. Today, a human has to fund it. MoltBot changes that.
>
> [Show agent loop starting with $10 of USDC]
>
> MoltBot's first move: park 80% in Aave V3 via a KeeperHub yield workflow. [Show Aave tx hash]
>
> Now MoltBot has a job: keep my Morpho position healthy. To check the health factor, it needs to call a paid workflow. [Show x402 payment going through] $0.01 USDC, settled on Base.
>
> [Show dashboard with 10+ runs, multiple yield actions, multiple job actions, all green]
>
> After 24 hours: MoltBot earned $0.42 in yield, spent $0.18 on x402 payments, completed 12 job actions, and never ran out of money.
>
> Every single decision is in KeeperHub's audit trail. Public. Replayable.
>
> MoltBot is the first paying customer of the keeper economy."

### 11.3 Channel strategy

| Channel | Audience | Content | Frequency |
|---|---|---|---|
| X (Twitter) | Crypto + agent builders | Daily devlogs, run snapshots, demo clips | Daily during build |
| Hacker News (Show HN) | Devs | "Show HN: An AI agent that funds itself by paying onchain keepers" | Once, on submission day |
| /r/ethdev, /r/rust, /r/solidity | Devs | Submission post + follow-up if HN traction | Once each |
| KeeperHub Discord | Hackathon judges, other builders | Devlogs in #builder channel, ask questions in #help | Daily |
| x402scan.com | x402 ecosystem | Implicit — our x402 payments will show up in their index | Automatic |
| DoraHacks | Hackathon judges + community | Submission, BUIDL updates, ask questions | Weekly |

### 11.4 First users (the first 10)

The hackathon doesn't need "users" in the SaaS sense. The audience is *judges*. But we want buzz.

| User # | Tactic |
|---|---|
| 1–3 | The KeeperHub team (tag them in Discord, send the demo privately) |
| 4–7 | Active x402/KeeperHub builders on X (DM the demo) |
| 8–10 | Anyone who interacts with our devlogs (reply to every comment) |

---

## 12. Presentation & Submission Strategy (Phase 3 §14)

### 12.1 The 5-minute demo script (for the video)

```
0:00–0:30  THE PROBLEM. "Every AI agent needs a wallet. Today, a human funds it. That's a single point of failure."
0:30–1:30  THE SETUP. "MoltBot is a Rust agent. It starts with $10 of USDC. Its first move is to park 80% in Aave V3." [Show Aave tx on Etherscan]
1:30–2:30  THE LOOP. "Every 60 seconds, MoltBot runs its job — keep my Morpho position healthy. To check the health factor, it pays a keeper via x402." [Show x402 payment in dashboard]
2:30–3:30  THE EVIDENCE. "After 24 hours, MoltBot has: 4 yield actions, 18 job actions, $0.42 earned, $0.18 spent. Net positive." [Show dashboard with all 22 runs]
3:30–4:30  THE DIFFERENTIATOR. "Every other hackathon submission is a KeeperHub workflow. MoltBot is a KeeperHub *customer* — it treats the marketplace as its economic body." [Show agent loop diagram]
4:30–5:00  THE CALL. "Open source. Rust crate coming to crates.io. PR open against KeeperHub org. Watch the agent live at the dashboard URL." [Show repo, dashboard, links]
```

### 12.2 Submission copy (draft, ~250 words)

> **MoltBot: the first paying customer of the KeeperHub marketplace.**
>
> MoltBot is a Rust autonomous agent that uses KeeperHub as its execution and earning layer. It holds a USDC balance on Base, parks idle funds in Aave V3 via a KeeperHub yield workflow, and pays for keeper work in real time via x402 whenever it needs to read onchain state, claim rewards, or move funds. Every economic decision is logged through KeeperHub's audit trail — the agent's full economic life is replayable from a single page.
>
> **Why it's useful:** every agentic SaaS that needs a wallet that pays its own way. Self-funding DAO treasuries. AI agents that own their own work loops.
>
> **KeeperHub surfaces used:** MCP server (call_workflow, search_workflows, get_execution_logs), x402 auto-pay, MPP (feature-flagged), audit trail, smart gas, gas sponsorship. 8 of 9 surfaces.
>
> **By the numbers (24h dry run):**
> - 22 onchain transactions, all successful
> - $0.42 yield earned, $0.18 spent on x402 payments
> - 0 human interventions after initial config
> - 4 yield actions, 18 job actions
>
> **Why it wins the grand prize:** every other submission is a KeeperHub workflow or a KeeperHub-backed agent. MoltBot is a KeeperHub *customer*. It's the first end-to-end demonstration of the marketplace flywheel — agent pays keeper, keeper earns, marketplace grows.
>
> **Open source:** github.com/[user]/moltbot. The Rust client for KeeperHub is shipped as a separate crate (`keeperhub-rs`) and a PR is open against the KeeperHub org for upstream integration.
>
> [Live dashboard] [Etherscan tx] [GitHub] [Demo video]

### 12.3 GitHub repo requirements

- README with 3+ screenshots (dashboard, audit trail, x402 payment in KeeperHub runs) above the fold
- DEMO.md with the live dashboard URL and a 30-second screen recording (gif)
- ARCHITECTURE.md (this document, slightly trimmed)
- SCORING.md explaining the rubric-by-rubric scoring
- LICENSE: MIT
- .env.example (no secrets in repo)
- `cargo test` runs cleanly
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- `cargo fmt --check` clean
- A `MoltBot-Live-2026-08-XX.json` snapshot of the agent's actual run history in the repo

### 12.4 Post-submission moves

- Aggregated stats post: "MoltBot ran for X hours, did Y transactions, earned $Z, spent $W"
- "What I learned building this" thread (generates engagement, signals seriousness)
- The crates.io publish of `keeperhub-rs` (signals long-term commitment)
- A merged PR to the KeeperHub org (signals community contribution)

---

## 13. The Bounty — Killer Side Quest

The hackathon has a $1,000 bounty for "Best Onboarding UX Improvement." While building MoltBot, the natural deliverable for this bounty *falls out of the work*:

**The `keeperhub-rs` crate IS the bounty deliverable.**

It's a:
- Starter template (Rust developers can use KeeperHub today)
- Tutorial (the README + the MoltBot source as a worked example)
- Clear teardown of where I got stuck (a "lessons learned" doc that becomes a PR comment or an issue)
- Optionally, a merged PR to the KeeperHub org adding a Rust adapter to their official SDK list

**Both prizes stack.** The grand prize is judged on the BUIDL. The bounty is judged on the contribution. They don't conflict.

---

## 14. Decision Matrix

| Decision | Choice | Reasoning |
|---|---|---|
| Product | Self-funding Rust agent on KeeperHub marketplace | Highest originality, full surface coverage |
| Stack | Rust (agent) + solidity (yield workflow helper) | User's preference; matches Rust gap in KeeperHub ecosystem |
| Yield strategy | Aave V3 supply on Base | Best-supported by KeeperHub, deepest liquidity, cheapest gas |
| Job | "Keep my Morpho position healthy" | Multi-step, auditable, economic value |
| Wallet | KeeperHub agentic wallet (primary) + self-custody path (sketched) | Demo-friendly, no cap issues for the demo; self-custody as roadmap |
| x402 client | Build in `keeperhub-rs` | Bounty deliverable, also MoltBot needs it |
| MCP transport | HTTP (not stdio) | Simpler, no local process; matches KeeperHub docs |
| Loop frequency | 60s | Good cadence for the dashboard, low cost |
| Dashboard stack | Axum + SQLite + vanilla JS | Minimum viable, easy to ship |
| Where to ship | crates.io (`keeperhub-rs`) + GitHub (MoltBot) | Maximizes impact, doubles as bounty |
| Submission copy framing | "First paying customer of the keeper economy" | Differentiates from every other submission |
| Video length | 90 seconds | Long enough to tell the story, short enough to watch |
| Pre-build usage | 24 days of doc-reading + scaffolding | Eliminates unknowns before the 17-day clock starts |
| Buffer | 1.5 days in the build phase, 2 days in pre-build | Insurance against slips |

---

## 15. Why This Wins — The Closing Argument

The KeeperHub hackathon is judging agents that *execute onchain*. The single most important thing a judge will ask is: **"Does this thing run, and would anyone use it?"**

Most submissions will be KeeperHub workflows. They run. They're useful. They use the surfaces. But they're not *novel* — they're demos of a platform that already has workflow templates.

A few submissions will be AI agents that use KeeperHub. They run. They use the surfaces. They might be novel. But they still need a human to fund them.

**MoltBot is the only submission where the agent funds itself, the only submission that treats the KeeperHub marketplace as its economic body, and the only submission that demonstrates the marketplace flywheel in action.** It's not a demo of KeeperHub. It's a demo of what happens when KeeperHub is the *substrate* for autonomous economic actors.

The judges will see a working Rust agent. They'll see real transactions. They'll see real x402 payments. They'll see a dashboard that any non-crypto person can understand. They'll see the audit trail and the yield earned and the payments made. And they'll think: *"This is what agentic commerce looks like."*

That's the win.
