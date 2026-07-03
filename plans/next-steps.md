# Next Steps & Iteration Plan

> Master plan for building MoltBot, broken into trackable iterations.
> Each iteration maps to a GitHub issue. Use this doc as the single source
> of truth for "what's next."

**Status:** 2/31 iterations complete (scaffold + setup checklist).
**Goal:** Ship grand prize + bounty by Aug 13, 2026.
**Working commit cadence:** one issue → one commit → one verification → next.

---

## Quick orientation

The build is structured as **four parallel tracks** running in dependency order:

1. **Track A — `keeperhub-rs` (the Rust client)**: standalone library that talks to KeeperHub. This is the bounty deliverable *and* the foundation MoltBot uses.
2. **Track B — KeeperHub workflows (the supply side)**: the Aave yield workflow and Morpho job workflow that MoltBot will *call*. Built in the visual builder.
3. **Track C — `moltbot` (the agent)**: the Rust binary that runs the loop, calls Track A's client, pays via x402, audits everything.
4. **Track D — Dashboard + demo**: the Axum audit-trail UI, the demo video, the submission copy.

Tracks A and B are independent (no blockers between them). Track C depends on A. Track D depends on C. **Parallelize A and B aggressively.**

---

## Issue list at a glance

| # | Title | Track | Priority | Estimate | Depends on |
|---|---|---|---|---|---|
| 1 | Run local setup checklist | ops | P0 | M | — |
| 2 | keeperhub-rs: implement `list_workflows` | A | P0 | M | #1 |
| 3 | keeperhub-rs: implement `call_workflow` (free) | A | P0 | M | #2 |
| 4 | keeperhub-rs: implement `get_execution_logs` | A | P0 | S | #2 |
| 5 | keeperhub-rs: x402 EIP-3009 builder | A | P0 | M | #1 |
| 6 | keeperhub-rs: `call_paid_workflow` with x402 auto-pay | A | P0 | L | #3, #5 |
| 7 | `keeperhub-rs`: Aave direct call via `execute_protocol_action` | A | P0 | S | #1, Aave integration |
| 7b | Build + publish one marketplace workflow (visual builder) | B | P0 | M | #1 |
| 8 | `keeperhub-rs`: Morpho direct call via `execute_protocol_action` + HF calc | A | P0 | S | #1, Morpho integration |
| 9 | moltbot: agent loop skeleton (60s tick) | C | P0 | M | #2 |
| 10 | moltbot: yield strategy (call Aave, parse) | C | P0 | M | #6, #7, #9 |
| 11 | moltbot: job system + Morpho impl | C | P0 | L | #6, #8, #9 |
| 11b | Configure Aave V3 + Morpho integrations in KeeperHub | ops | P0 | S | #1 |
| 12 | moltbot: safe-mode (low-balance detection) | C | P0 | S | #9 |
| 13 | keeperhub-rs: implement `search_workflows` | A | P1 | S | #2 |
| 14 | moltbot: SQLite audit log | C | P1 | M | #9 |
| 15 | moltbot: Axum dashboard rendering audit log | D | P1 | L | #14 |
| 16 | Wire Aave yield into loop, first onchain tx | C | P0 | M | #10 |
| 17 | Wire Morpho job into loop, real decision | C | P0 | M | #11 |
| 18 | x402 end-to-end on Sepolia testnet | A/C | P0 | M | #6 |
| 19 | 24h stress test (20+ onchain txs) | C | P0 | XL | #16, #17, #18 |
| 20 | Record 90-second demo video | D | P0 | L | #19 |
| 21 | Submission copy + README polish | D | P0 | M | #19 |
| 22 | Submit to DoraHacks (target: Aug 9) | ops | P0 | S | #20, #21 |
| 23 | Telegram alert integration | C | P1 | S | #12 |
| 24 | Pre-x402 balance check + skip-when-low | C | P1 | S | #6, #12 |
| 25 | keeperhub-rs: polish for crates.io publish | A | P0 | M | #6, #13 |
| 26 | keeperhub-rs: publish to crates.io | A | P0 | S | #25 |
| 27 | Open PR to KeeperHub org (Rust adapter) | A | P1 | M | #26 |
| 28 | Tutorial: "Build your first Rust agent on KeeperHub" | A | P1 | L | #26 |
| 29 | Marketing: aggregated stats post | ops | P1 | S | #19 |
| 30 | Marketing: "What I learned" thread | ops | P2 | S | #22 |
| 31 | PR follow-up + community engagement | ops | P2 | S | #27 |

**Size scale:** S < 2h · M 2-6h · L 6-16h · XL 16+h
**Total estimate:** ~22-28 working days solo. We have 41 days (24 pre-build + 17 build). **Comfortable buffer.**

---

## The 5 dependency chains

```
Chain 1 (keeperhub-rs core):     #1 → #2 → #3 → #4, #13
                                     ↘ #5 → #6
Chain 2 (KeeperHub workflows):   #1 → #7b (and #11b in parallel)
Chain 3 (moltbot agent):         #2 → #9 → #10, #11, #12
                                            ↘ #14 → #15
Chain 4 (build + demo):          #6, #7, #8, #7b, #10, #11 → #16, #17, #18 → #19 → #20, #21 → #22
Chain 5 (bounty):                #6, #13 → #25 → #26 → #27, #28
```

**Critical paths:**
- **Path A** (grand prize): #1 → #2 → #9 → #10 → #16 → #19 → #20 → #22 = ~13-15 days
- **Path B** (bounty): #1 → #2 → #3 → #5 → #6 → #25 → #26 = ~3-4 days

These can run in parallel. Pre-build #1 (setup) blocks everything but is fast.

---

## Iteration cadence (how to work through them)

For each issue:

1. **Open the issue** (or pick from this doc)
2. **Read the full card** below
3. **Branch** off `main`: `git checkout -b iter/<number>-<short-name>`
4. **Implement** the acceptance criteria
5. **Verify**: run the "Done =" check
6. **Commit**: `git commit -m "iter #<n>: <title>"`
7. **Push branch** + open PR (or merge directly to main for solo)
8. **Update issue**: close it, mark `done` label
9. **Move on** to the next

**Batch work by track, not by issue number.** All A-track issues can mostly be done back-to-back in one sitting. All B-track issues happen in the KeeperHub UI (no Rust involved).

**Standups (for yourself):** at the end of each work session, write 3 lines:
- What I shipped today
- What I'm doing next
- What's blocking me (if anything)

Save as `plans/daily/<date>.md` if you want a record.

---

## Issue cards

### #1 — Run local setup checklist

**Track:** ops · **Priority:** P0 · **Estimate:** M (~50 min) · **Blocks:** everything

**Why:** Without accounts, API keys, and a funded testnet wallet, none of the code can be tested against real KeeperHub. This is the on-ramp.

**What to do:** Open `plans/setup-verified.md` and tick every box. Save the final completion date in the doc.

**Acceptance criteria:**
- [ ] KeeperHub account + org + API key obtained
- [ ] Sepolia testnet wallet funded with 0.01 ETH
- [ ] `kh doctor` passes
- [ ] `kh workflow list` works
- [ ] MCP server connected (or `curl` test passes)
- [ ] Visual builder workflow created and run manually
- [ ] Agentic wallet installed + `~/.keeperhub/wallet.json` backed up
- [ ] `auto_approve_max_usd: 0.50` set in `safety.json`
- [ ] x402 paid call tested end-to-end (small amount)
- [ ] Rust toolchain (`cargo --version`) confirmed
- [ ] `cargo check` from the workspace root passes

**Done =** Every box in `setup-verified.md` is ticked, the doc has a completion date, and your API key + wallet address are in 1Password.

---

### #2 — `keeperhub-rs`: implement `list_workflows`

**Track:** A · **Priority:** P0 · **Estimate:** M (3h) · **Depends on:** #1

**Why:** The agent's primary discovery mechanism. Without it, we can't find or call any workflow.

**What to do:**
- Replace the `Err(Error::Internal(...))` stub in `crates/keeperhub-rs/src/mcp.rs::list_workflows` with a real implementation
- POST a JSON-RPC `tools/call` request to the MCP endpoint
- Parse the response into `Vec<Workflow>`
- Add integration tests against the live MCP server (use a feature flag for the real test)

**Acceptance criteria:**
- [ ] `list_workflows().await` returns `Vec<Workflow>` populated from the live API
- [ ] Empty org returns empty vec (not error)
- [ ] Errors map cleanly to `Error::Api { status, message }` for 4xx/5xx
- [ ] Integration test: `cargo test --features live-mcp` passes against a real org
- [ ] `cargo run -p keeperhub-rs --example list_workflows` prints the workflow list

**Done =** The example prints real workflows from your org.

---

### #3 — `keeperhub-rs`: implement `call_workflow` (free)

**Track:** A · **Priority:** P0 · **Estimate:** M (2h) · **Depends on:** #2

**Why:** The agent's primary execution mechanism. We need to call a workflow and get a result.

**What to do:**
- Add `McpClient::call_workflow(slug, inputs).await -> Result<JsonValue>`
- Returns the workflow's output for free workflows
- Returns a `402` with a `PaymentChallenge` for paid workflows (we handle the actual payment in #6)

**Acceptance criteria:**
- [ ] Free workflow calls return parsed JSON output
- [ ] Paid workflow calls return `Err(Error::X402Unpaid(challenge))`
- [ ] Errors map cleanly to `Error::Api` for 4xx/5xx
- [ ] Integration test passes

**Done =** You can call a free KeeperHub workflow from Rust.

---

### #4 — `keeperhub-rs`: implement `get_execution_logs`

**Track:** A · **Priority:** P0 · **Estimate:** S (2h) · **Depends on:** #2

**Why:** The agent's primary observability. Every action must end up in the audit trail.

**What to do:**
- Add `McpClient::get_execution_logs(execution_id).await -> Result<ExecutionLogs>`
- Parse into the `ExecutionLogs` type already in `types.rs`

**Acceptance criteria:**
- [ ] Returns full structured logs (LogEntry per node)
- [ ] Empty logs handled
- [ ] Integration test passes

**Done =** A previously-executed workflow's logs can be retrieved in full.

---

### #5 — `keeperhub-rs`: x402 EIP-3009 builder

**Track:** A · **Priority:** P0 · **Estimate:** M (4h) · **Depends on:** #1

**Why:** The auto-pay mechanism. Without signing EIP-3009, the agent can't pay for paid workflows.

**What to do:**
- Implement `parse_challenge` in `x402.rs` (already has a stub)
- Implement `build_payment_header` (returns base64-encoded JSON of `{signature, authorization}`)
- Add a `PaymentSigner` trait that abstracts the signing backend
- Add a `KeeperHubProxySigner` that calls the KeeperHub signing endpoint (Turnkey-mediated) — this is what avoids holding private keys locally
- Add a `LocalSigner` (with `alloy-rs`) as a fallback for self-custody path (skeleton only — full impl is post-hackathon)

**Acceptance criteria:**
- [ ] `parse_challenge` correctly parses a real 402 body
- [ ] `build_payment_header` produces a valid x402 `X-PAYMENT` header
- [ ] `PaymentSigner` trait is well-defined
- [ ] `KeeperHubProxySigner` at minimum constructs the right request shape
- [ ] Unit tests for the typed-data hash

**Done =** A 402 challenge from a real paid workflow can be parsed and converted into a signed payment header.

---

### #6 — `keeperhub-rs`: `call_paid_workflow` with x402 auto-pay

**Track:** A · **Priority:** P0 · **Estimate:** L (6h) · **Depends on:** #3, #5

**Why:** This is the "paying customer" mechanism. The whole pitch.

**What to do:**
- Add `McpClient::call_paid_workflow(slug, inputs, signer).await -> Result<JsonValue>`
- On 402, parse the challenge, ask the signer for a payment, retry with the X-PAYMENT header
- On 200, parse the workflow result
- Surface all errors clearly

**Acceptance criteria:**
- [ ] Free workflow calls work (no payment attempted)
- [ ] Paid workflow calls successfully auto-pay and return the result
- [ ] Insufficient-balance error maps to `Error::X402Unpaid`
- [ ] Retry on 402 is automatic and idempotent
- [ ] Integration test on Sepolia testnet passes (small $0.01 call)

**Done =** A real paid workflow can be called from Rust with auto-pay working.

---

### #7 — `keeperhub-rs`: Aave direct call via `execute_protocol_action`

**Track:** A · **Priority:** P0 · **Estimate:** S (1h) · **Depends on:** #1, Aave V3 integration configured in KeeperHub (#11b)

**Why:** The agent's own yield strategy. We chose direct `execute_protocol_action` over a visual-builder workflow because the wrapper adds nothing — we just need a single Aave supply/withdraw call with our own decision logic. The "publish a workflow to the marketplace" use case is handled by #7b.

**What to do:**
- Add `McpClient::execute_protocol_action(action_type, params).await -> Result<JsonValue>` to `keeperhub-rs`
- Wrap `McpClient::tools_call("execute_protocol_action", args)` under the hood
- Add typed helpers `aave_supply(network, asset, amount, on_behalf_of)` and `aave_withdraw(network, asset, amount, to)` that call through `execute_protocol_action`
- Integration test on Sepolia testnet (no real Aave on Sepolia — use `network=11155111` and a test contract, or mock at the MCP layer with `live-mcp` feature flag)

**Acceptance criteria:**
- [ ] `execute_protocol_action` returns parsed JSON for a valid action
- [ ] Aave supply/withdraw typed helpers work
- [ ] Integration test: `cargo test --features live-mcp` passes against the real MCP server
- [ ] `cargo run -p keeperhub-rs --example aave_supply` does a real testnet supply

**Done =** Rust code that supplies/withdraws from Aave V3 via a single typed call.

---

### #7b — Build + publish one marketplace workflow (visual builder)

**Track:** B · **Priority:** P0 · **Estimate:** M (3h) · **Depends on:** #1

**Why:** The supply side of the "first paying customer" pitch. We *also* publish a workflow to the KeeperHub marketplace — so we're not just consuming, we're participating. Doubles as the bounty "ecosystem contribution" proof.

**What to do:**
- Pick the workflow (suggested: "Aave + Morpho portfolio risk check" — given any wallet, returns HF, collateral, debt, suggested action)
- Build it in the visual builder with:
  - Manual trigger
  - Input: `wallet` (string)
  - Actions: `morpho/vault-balance` + `aave-v3/get-user-account-data` + a small condition node
  - Returns: structured JSON
- Publish to marketplace with `list_workflow` MCP tool, price `$0.05` per call
- Test the listing by calling it from `keeperhub-rs` via `search_workflows` + `call_workflow` (x402 auto-pay kicks in — this exercises #5/#6 too)

**Acceptance criteria:**
- [ ] Workflow is listed in the marketplace (slug is set)
- [ ] `search_workflows` finds it by tag/category
- [ ] `call_workflow` from our own `keeperhub-rs` succeeds and pays the x402 amount
- [ ] Listed workflow appears in `x402scan`
- [ ] Listed workflow's runs panel shows clean runs

**Done =** A listed KeeperHub workflow that other agents can pay for, called at least once by MoltBot itself.

---

### #8 — `keeperhub-rs`: Morpho direct call via `execute_protocol_action` + HF calc

**Track:** A · **Priority:** P0 · **Estimate:** S (1h) · **Depends on:** #1, Morpho integration configured in KeeperHub (#11b)

**Why:** The agent's own job (position monitoring). Direct call is fine — health-factor math is a Rust function, not a workflow.

**What to do:**
- Add typed helpers `morpho_vault_balance(network, vault, account)` and `morpho_market_position(network, market_id, user)`
- Add `compute_health_factor(collateral_usd, debt_usd, liq_threshold) -> f64` as a pure Rust function
- Integration tests

**Acceptance criteria:**
- [ ] Morpho balance helpers work
- [ ] `compute_health_factor` unit-tested (3 cases: healthy, at-risk, underwater)
- [ ] Integration test on Base testnet (or mock with `live-mcp`)

**Done =** Rust code that reads a Morpho position and computes its health factor.

---

### #9 — `moltbot`: agent loop skeleton (60s tick)

**Track:** C · **Priority:** P0 · **Estimate:** M (4h) · **Depends on:** #2

**Why:** The core loop. Without it, nothing else runs.

**What to do:**
- Replace the placeholder `crates/moltbot/src/main.rs` with a real loop
- Add `crates/moltbot/src/config.rs` (AgentConfig loading from TOML + env)
- Add `crates/moltbot/src/loop.rs` (the `tick()` function, called every 60s)
- Add `crates/moltbot/src/state.rs` (in-memory state: balance, last action, safe mode)
- For now, the loop just logs "iteration N started" and the current USDC balance

**Acceptance criteria:**
- [ ] `cargo run -p moltbot` starts and runs forever
- [ ] Every 60s, logs an iteration line with timestamp + USDC balance
- [ ] Graceful shutdown on SIGINT
- [ ] Config loaded from `moltbot.toml`
- [ ] Missing config errors clearly

**Done =** Binary runs forever, logs every 60s, shuts down cleanly.

---

### #10 — `moltbot`: yield strategy (call Aave, parse)

**Track:** C · **Priority:** P0 · **Estimate:** M (3h) · **Depends on:** #6, #7, #9

**Why:** The "self-funding" part. The agent decides when to park in Aave and calls the workflow.

**What to do:**
- Add `crates/moltbot/src/yield_strategy.rs`
- Decision: if USDC > park_threshold (default $50), call Aave supply; if USDC < withdraw_threshold (default $20), call Aave withdraw
- Parse the result, log the tx hash
- Update the in-memory state with the new balance

**Acceptance criteria:**
- [ ] On balance > $50, agent calls Aave supply, logs the tx hash
- [ ] On balance < $20, agent calls Aave withdraw, logs the tx hash
- [ ] On balance in between, no action
- [ ] Decision logic is unit-tested with mocked balances

**Done =** Agent can park and withdraw from Aave autonomously.

---

### #11 — `moltbot`: job system + Morpho impl

**Track:** C · **Priority:** P0 · **Estimate:** L (5h) · **Depends on:** #6, #8, #9

**Why:** The job the agent does. Pluggable so we can add more jobs later.

**What to do:**
- Add `crates/moltbot/src/job.rs` (Job trait)
- Add `crates/moltbot/src/jobs/morpho_health.rs` (the Morpho job)
- The job uses the typed helpers from #8 to read position + compute HF, decides whether to call `morpho/supply-collateral` via `execute_protocol_action`
- Add a `Job` enum and dispatcher

**Acceptance criteria:**
- [ ] Job trait is well-defined: `name()`, `tick()`, `should_run(state) -> bool`
- [ ] Morpho job runs on every tick
- [ ] Decision logic is unit-tested
- [ ] Adding a second job (e.g., `price_alert`) requires <50 lines

**Done =** Agent monitors a Morpho position and auto-collateralizes on low health.

---

### #11b — Configure Aave V3 + Morpho integrations in KeeperHub

**Track:** ops · **Priority:** P0 · **Estimate:** S (15 min) · **Depends on:** #1

**Why:** `execute_protocol_action` with `requiresCredentials: true` (Aave, Morpho) won't work until the org-level integration is configured. One-time setup.

**What to do:**
- In KeeperHub app: Integrations → Add → search "Aave V3" → connect
- Same for Morpho
- Verify with `list_integrations` MCP tool — both should appear
- Note the integration IDs in `keeperhub-docs-summary.md`

**Acceptance criteria:**
- [ ] Aave V3 integration is configured
- [ ] Morpho integration is configured
- [ ] `list_integrations` shows both
- [ ] A test call to `execute_protocol_action("aave-v3/get-user-account-data", ...)` returns successfully

**Done =** Aave + Morpho integrations live in the org. Required for #7, #8, #10, #11.

---

### #12 — `moltbot`: safe-mode (low-balance detection)

**Track:** C · **Priority:** P0 · **Estimate:** S (1h) · **Depends on:** #9

**Why:** Safety. If the agent runs out of USDC, it should stop trying to pay and alert.

**What to do:**
- Add `crates/moltbot/src/safe_mode.rs`
- On every tick, check if USDC balance < safe_mode_threshold (default $5)
- If yes, set `state.safe_mode = true`, send a Telegram alert (deferred to #23 if Telegram not yet integrated), and skip all paid actions
- If balance recovers above threshold, exit safe mode and alert

**Acceptance criteria:**
- [ ] On balance < $5, agent enters safe mode and logs a clear message
- [ ] Paid actions are skipped while in safe mode
- [ ] State is correctly restored when balance recovers
- [ ] Unit-tested

**Done =** Agent self-protects against running out of money.

---

### #13 — `keeperhub-rs`: implement `search_workflows`

**Track:** A · **Priority:** P1 · **Estimate:** S (1h) · **Depends on:** #2

**Why:** Discovery for the agent at runtime. The agent needs to find the right workflow by name, not just hard-code slugs.

**What to do:**
- Add `McpClient::search_workflows(query, category, tag).await -> Result<Vec<Workflow>>`
- Map to the `search_workflows` MCP tool

**Acceptance criteria:**
- [ ] Free-text query returns matching workflows
- [ ] Category filter works
- [ ] Tag filter works
- [ ] Integration test passes

**Done =** Agent can find workflows by name at runtime.

---

### #14 — `moltbot`: SQLite audit log

**Track:** C · **Priority:** P1 · **Estimate:** M (3h) · **Depends on:** #9

**Why:** The dashboard needs a local data source (KeeperHub's runs panel is the source of truth, but a local cache makes the dashboard fast).

**What to do:**
- Add `crates/moltbot/src/audit.rs`
- Use `sqlx` with SQLite
- Schema: `runs(id, started_at, ended_at, status, kind)`, `actions(id, run_id, kind, tx_hash, x402_payment, ...)`, `x402_payments(id, action_id, amount, asset, chain, tx_hash)`
- On every loop iteration, write a run record
- On every action, write an action record
- On every x402 payment, write a payment record

**Acceptance criteria:**
- [ ] Schema migration runs cleanly on first start
- [ ] Every loop iteration persists a run record
- [ ] Every action persists an action record
- [ ] Every x402 payment persists a payment record
- [ ] All writes are wrapped in transactions (atomic per run)

**Done =** Local SQLite has a complete record of every agent action.

---

### #15 — `moltbot`: Axum dashboard rendering audit log

**Track:** D · **Priority:** P1 · **Estimate:** L (5h) · **Depends on:** #14

**Why:** The demo's visual centerpiece. Judges see the audit trail come alive.

**What to do:**
- Add `crates/moltbot/src/dashboard.rs`
- Axum server on `localhost:3030` (or configurable)
- Routes:
  - `GET /` — HTML dashboard
  - `GET /api/runs` — JSON list of recent runs
  - `GET /api/runs/:id` — JSON single run detail
  - `GET /api/stats` — aggregate stats (total earned, total spent, net)
- Static `index.html` + `app.js` + `style.css` in `crates/moltbot/static/`
- The page polls `/api/runs` every 10s and renders a live view

**Acceptance criteria:**
- [ ] Dashboard loads at `http://localhost:3030`
- [ ] Shows recent runs with tx hashes (clickable to Etherscan)
- [ ] Shows aggregate stats: total earned, total spent, current balance
- [ ] Updates every 10s without page refresh
- [ ] Looks clean enough for a demo video (Tailwind or hand-rolled CSS)

**Done =** A live dashboard showing the agent's economic life.

---

### #16 — Wire Aave yield into loop, first onchain tx

**Track:** C · **Priority:** P0 · **Estimate:** M (3h) · **Depends on:** #10

**Why:** The moment MoltBot moves real value onchain. The proof of execution.

**What to do:**
- Fund the agent's wallet with $50 USDC on Base mainnet (use a CEX or your own wallet)
- Run the agent
- Confirm: agent detects balance > threshold, calls Aave supply, USDC moves to Aave, balance drops accordingly
- Note the tx hash

**Acceptance criteria:**
- [ ] Agent makes a real Aave supply tx
- [ ] The tx hash is in the audit log
- [ ] The x402 payment for the keeper call is in the audit log
- [ ] Dashboard shows the run

**Done =** A real onchain tx, attributable to MoltBot, visible in the dashboard.

---

### #17 — Wire Morpho job into loop, real decision

**Track:** C · **Priority:** P0 · **Estimate:** M (3h) · **Depends on:** #11

**Why:** The moment the agent does useful work. Proof of agency.

**What to do:**
- Set up a small Morpho position with intentionally low health factor (e.g., supply $100 of wstETH, borrow $70 of USDC, then drop ETH price ~10%)
- Run the agent
- Confirm: agent detects HF < 1.3, calls Morpho collateralize, position health restored
- Note the tx hashes

**Acceptance criteria:**
- [ ] Agent detects low HF
- [ ] Agent calls the collateralize workflow
- [ ] The HF is restored above 1.3
- [ ] Tx hashes in audit log

**Done =** A real agent decision, executed onchain, visible in the dashboard.

---

### #18 — x402 end-to-end on Sepolia testnet

**Track:** A/C · **Priority:** P0 · **Estimate:** M (2h) · **Depends on:** #6

**Why:** Verify the x402 auto-pay works against a real chain before going to mainnet.

**What to do:**
- Find or create a paid workflow on Sepolia
- Run the agent against it
- Confirm: 402 returned, signed, paid, result returned
- Note the x402scan entry

**Acceptance criteria:**
- [ ] x402 payment settles on Sepolia
- [ ] x402scan shows the payment entry
- [ ] Workflow returns the expected result after payment
- [ ] Agent logs the payment

**Done =** A real x402 payment visible on x402scan.

---

### #19 — 24h stress test (20+ onchain txs)

**Track:** C · **Priority:** P0 · **Estimate:** XL (24h run + 2h setup) · **Depends on:** #16, #17, #18

**Why:** The "this thing actually works" demo. Judges need to see sustained, real execution.

**What to do:**
- Run the agent for 24+ hours on Base mainnet (or testnet if mainnet is too expensive)
- Let it loop normally — yield + job + audit
- At the end, snapshot the dashboard, save the SQLite DB
- Verify 20+ onchain txs, 100+ audit log entries, no crashes

**Acceptance criteria:**
- [ ] Agent ran for 24+ hours without crashing
- [ ] 20+ onchain txs in the audit log
- [ ] x402 payments visible on x402scan
- [ ] Dashboard renders all of it
- [ ] SQLite DB snapshot saved to `plans/24h-run-<date>.db`

**Done =** A clean 24h run with 20+ real txs.

---

### #20 — Record 90-second demo video

**Track:** D · **Priority:** P0 · **Estimate:** L (6h) · **Depends on:** #19

**Why:** The submission's most important asset. Judges watch this first.

**What to do:**
- Storyboard per `plans/moltbot-deep-research.md` §12.1
- Record the 5 segments: problem, setup, loop, evidence, differentiator, call
- Edit to 90s
- Upload as unlisted YouTube
- Embed in submission

**Acceptance criteria:**
- [ ] Video is 90-120s
- [ ] Shows a real onchain tx (Etherscan visible)
- [ ] Shows an x402 payment (x402scan or KeeperHub audit visible)
- [ ] Shows the dashboard with multiple runs
- [ ] Has a clear call-to-action (GitHub, dashboard link)
- [ ] Audio is clean (or muted with on-screen text)

**Done =** Unlisted YouTube link, embedded in the submission.

---

### #21 — Submission copy + README polish

**Track:** D · **Priority:** P0 · **Estimate:** M (3h) · **Depends on:** #19

**Why:** The submission is the first thing judges read after the video.

**What to do:**
- Write the 250-word submission copy per `plans/moltbot-deep-research.md` §12.2
- Polish the README (screenshots, GIF, link to video)
- Write `DEMO.md` with reproduction steps
- Write `ARCHITECTURE.md` (slim version of the research doc)
- Write `SCORING.md` mapping features to rubric
- Add `LICENSE` (MIT)
- Add `.env.example` (no secrets)

**Acceptance criteria:**
- [ ] Submission copy is 250 words, tells the story in 2-3 sentences
- [ ] README has 3+ screenshots above the fold
- [ ] DEMO.md has a one-command reproduction
- [ ] ARCHITECTURE.md has the agent loop diagram
- [ ] SCORING.md has the rubric-by-rubric breakdown
- [ ] Repo is publicly visible (when pushed)

**Done =** Everything judges need to understand the project is in the repo.

---

### #22 — Submit to DoraHacks (target: Aug 9)

**Track:** ops · **Priority:** P0 · **Estimate:** S (30min) · **Depends on:** #20, #21

**Why:** Deadline is Aug 13 12:00 UTC+2. Submitting Aug 9 gives 4 days of buffer.

**What to do:**
- Go to https://dorahacks.io/hackathon/agents-onchain/buidl
- Click "Submit BUIDL"
- Fill in: title, description (paste submission copy), GitHub link, demo video link, transaction link
- Required: GitHub link, demo video, link to a tx your agent executed

**Acceptance criteria:**
- [ ] Submission live on DoraHacks
- [ ] All three required fields filled (GitHub, video, tx link)
- [ ] Submission is publicly visible
- [ ] Confirmation email received (if applicable)

**Done =** Submission live, before Aug 13 deadline.

---

### #23 — Telegram alert integration

**Track:** C · **Priority:** P1 · **Estimate:** S (1h) · **Depends on:** #12

**Why:** Safe-mode alerts need a way to reach the operator. Telegram is the easiest.

**What to do:**
- Set up a Telegram bot (BotFather, 1 minute)
- Add the bot's chat_id to config
- On safe mode entry/exit, send a Telegram message via KeeperHub's Telegram plugin
- On every x402 payment, optionally send a small alert

**Acceptance criteria:**
- [ ] Safe mode entry triggers a Telegram message
- [ ] Safe mode exit triggers a Telegram message
- [ ] Messages are formatted cleanly

**Done =** Operator gets Telegram alerts on safe mode events.

---

### #24 — Pre-x402 balance check + skip-when-low

**Track:** C · **Priority:** P1 · **Estimate:** S (1h) · **Depends on:** #6, #12

**Why:** Defense in depth. Before every paid call, check balance and skip if too low.

**What to do:**
- Add a check in the loop: before calling `call_paid_workflow`, verify balance > max_x402_payment (configurable, default $0.10)
- If not, log and skip the call
- Don't enter safe mode (that's reserved for < $5 floor)

**Acceptance criteria:**
- [ ] Pre-call balance check in place
- [ ] Skip-when-low logs clearly
- [ ] Doesn't trip safe mode

**Done =** No accidental 402 prompts for high-cost calls.

---

### #25 — `keeperhub-rs`: polish for crates.io publish

**Track:** A · **Priority:** P0 · **Estimate:** M (4h) · **Depends on:** #6, #13

**Why:** The bounty deliverable needs to be a real, polished crate. Not a hackathon prototype.

**What to do:**
- Full doc comments on every public item
- `#![deny(missing_docs)]` clean
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- `cargo fmt` clean
- 80%+ test coverage on the MCP module
- At least one example that runs end-to-end
- Cargo.toml metadata complete: keywords, categories, description
- License file (MIT)
- README on the crate is the keeperhub-rs/README.md we already have

**Acceptance criteria:**
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo doc --no-deps` produces clean docs
- [ ] `cargo package` succeeds
- [ ] Test coverage on mcp module is 80%+

**Done =** Crate is publishable.

---

### #26 — `keeperhub-rs`: publish to crates.io

**Track:** A · **Priority:** P0 · **Estimate:** S (15min) · **Depends on:** #25

**Why:** The bounty explicitly rewards ecosystem contributions. Publishing to crates.io is the proof.

**What to do:**
- Get a crates.io API token (https://crates.io/me)
- `cargo login <token>`
- `cargo publish -p keeperhub-rs`
- Verify the crate is live

**Acceptance criteria:**
- [ ] Crate is live on https://crates.io/crates/keeperhub-rs
- [ ] README renders correctly on the crate page
- [ ] Docs build and are accessible

**Done =** Live crate on crates.io.

---

### #27 — Open PR to KeeperHub org (Rust adapter)

**Track:** A · **Priority:** P1 · **Estimate:** M (2h) · **Depends on:** #26

**Why:** The "merged PR" path of the bounty deliverable. Strong signal of community contribution.

**What to do:**
- Fork https://github.com/KeeperHub/keeperhub
- Add a `crates/keeperhub-rs` link or a "Rust SDK" section to their README
- Add a docs page for the Rust integration
- Open the PR with a clear description

**Acceptance criteria:**
- [ ] PR opened against KeeperHub/keeperhub
- [ ] PR description explains the integration, links to crates.io
- [ ] PR is reviewable (not WIP)

**Done =** Open PR on KeeperHub repo.

---

### #28 — Tutorial: "Build your first Rust agent on KeeperHub"

**Track:** A · **Priority:** P1 · **Estimate:** L (4h) · **Depends on:** #26

**Why:** Onboarding improvements are explicitly part of the bounty. A tutorial is a strong contribution.

**What to do:**
- Write a `docs/tutorials/rust-agent.md` (or similar) for the KeeperHub docs site
- Walk through building a minimal Rust agent that calls a workflow
- Reference the keeperhub-rs crate
- Include the full source

**Acceptance criteria:**
- [ ] Tutorial is 500-1000 words
- [ ] Has a copy-pasteable code example
- [ ] Builds and runs end-to-end
- [ ] Submitted as a PR to KeeperHub docs

**Done =** Tutorial PR opened.

---

### #29 — Marketing: aggregated stats post

**Track:** ops · **Priority:** P1 · **Estimate:** S (1h) · **Depends on:** #19

**Why:** "Look at this thing actually work" is the most viral content. Numbers > narrative.

**What to do:**
- Write a post: "MoltBot ran for 24h. It did X onchain txs, earned $Y in yield, spent $Z on x402, and never ran out of money."
- Include dashboard screenshots
- Post on X, /r/ethdev, /r/rust, KeeperHub Discord, Hacker News (Show HN)
- Tag KeeperHub, @daborahacks, key x402 accounts

**Acceptance criteria:**
- [ ] Post is live on at least 3 channels
- [ ] Includes the dashboard URL
- [ ] Includes a link to the GitHub repo

**Done =** Live posts on 3+ channels.

---

### #30 — Marketing: "What I learned" thread

**Track:** ops · **Priority:** P2 · **Estimate:** S (1h) · **Depends on:** #22

**Why:** Engagement post. Generates discussion, signals seriousness.

**What to do:**
- Write a thread: 5-7 things I learned building an autonomous onchain agent
- Include one specific gotcha (e.g., x402 ask-tier behavior, KeeperHub MCP quirks)
- End with a link to the repo

**Acceptance criteria:**
- [ ] Thread is 5-7 tweets long
- [ ] Has at least one specific, non-obvious lesson
- [ ] Ends with a CTA

**Done =** Thread posted on X.

---

### #31 — PR follow-up + community engagement

**Track:** ops · **Priority:** P2 · **Estimate:** S (ongoing) · **Depends on:** #27

**Why:** Maintain momentum. Respond to issues, merge suggestions, post updates.

**What to do:**
- Watch for issues on the repo
- Respond to PRs
- Post weekly progress updates
- Engage in KeeperHub Discord

**Acceptance criteria:**
- [ ] No open issues older than 7 days without a response
- [ ] Weekly progress update posted

**Done =** Ongoing.

---

## Risk-mitigated iteration order

**Week 1 (Jul 3-9): #1, #2, #3, #4**
- Get the toolchain verified, get keeperhub-rs talking to the real MCP server. Critical foundation.

**Week 2 (Jul 10-16): #5, #6, #13, #7, #8**
- x402 builder + auto-pay in parallel with the two KeeperHub workflows. Two tracks in parallel.

**Week 3 (Jul 17-23): #9, #10, #11, #12**
- Agent loop comes alive. Yield + job + safe mode.

**Week 4 (Jul 24-30): #14, #15, #16, #17**
- Audit log + dashboard. First real onchain txs. Build phase opens Jul 27.

**Week 5 (Jul 31-Aug 6): #18, #19, #23, #24**
- x402 end-to-end, 24h stress test, Telegram alerts, balance pre-check.

**Week 6 (Aug 7-13): #20, #21, #22, #25, #26, #27, #28, #29, #30, #31**
- Demo video, submission copy, submit (target Aug 9), bounty publish, marketing.

---

## When to stop and ship

**The "ship now" decision tree:**

- If by Aug 6, the agent has run for 24h with 10+ onchain txs and the dashboard works → ship on schedule.
- If by Aug 6, the demo video isn't done → drop scope. Submit a working v0 with the video, defer polish.
- If by Aug 6, the x402 auto-pay is broken on mainnet → fall back to Sepolia testnet for the demo. Note in submission.
- If by Aug 6, the bounty crate isn't publishable → publish the existing keeperhub-rs anyway with a "0.1.0" tag.

**No silent failures.** Better to ship a working v0 with caveats than a perfect v1 that never lands.

---

## What we are NOT building (called out explicitly)

- ❌ LLM-driven decision-making (use hard-coded thresholds)
- ❌ Multi-chain support (Base + Sepolia only)
- ❌ More than 2 yield strategies (Aave only)
- ❌ More than 2 jobs (Morpho + price alert)
- ❌ Self-custody wallet path (sketched only)
- ❌ Real-time dashboard updates (10s polling is fine)
- ❌ Mobile UI
- ❌ User authentication
- ❌ Multi-MoltBot orchestration
- ❌ EIP-8004 identity registration (cool but not worth the time)

**Each of these has an explicit "no" so we don't drift. If a "yes" emerges during build, write a new issue first.**

---

## Done criteria for the whole project

The project is **done** when:

- [ ] Submission is live on DoraHacks
- [ ] All P0 issues are closed
- [ ] Demo video is up
- [ ] Repo is public and clean
- [ ] `keeperhub-rs` is on crates.io
- [ ] At least one marketing post is up
- [ ] No silent TODOs in the code
- [ ] The author would be comfortable sharing the repo on their resume

We hit this by **Aug 13, 12:00 UTC+2.** Comfortable buffer.
