#!/bin/bash
# Creates the 31 MoltBot GitHub issues.
# Run from any directory; uses GH_TOKEN env var.
set -e

export GH_TOKEN="${GH_TOKEN:?must be set}"
REPO="AduAkorful/moltbot"
MILESTONE_PRE="Phase 4: Pre-build"
MILESTONE_BUILD="Phase 5: Build + Demo"
MILESTONE_BOUNTY="Bounty: Onboarding UX"
MILESTONE_POST="Phase 6: Post-submit"

declare -a URLS

create_issue() {
  local num="$1"
  local title="$2"
  local body="$3"
  local labels="$4"
  local milestone="$5"
  local url
  url=$(gh issue create --repo "$REPO" --title "$title" --body "$body" --label "$labels" --milestone "$milestone" 2>&1)
  if [[ "$url" == https://github.com/* ]]; then
    URLS+=("$num: $url")
    echo "  ✓ #$num $(echo "$title" | cut -c1-60)..."
  else
    echo "  ✗ #$num FAILED: $url"
    return 1
  fi
}

echo "=== Creating 31 issues ==="
echo ""

# ============ #1 ============
create_issue 1 "#1 — Run local setup checklist" "$(cat <<'EOF'
## Why
Without accounts, API keys, and a funded testnet wallet, none of the code can be tested against real KeeperHub. This is the on-ramp that blocks all other work.

## What to do
- Open `plans/setup-verified.md` and tick every box (A through J)
- Save the final completion date in the doc
- Store the API key + wallet address in 1Password

## Acceptance criteria
- [ ] KeeperHub account + org + API key obtained (`kh_` prefix)
- [ ] Sepolia testnet wallet funded with 0.01 ETH
- [ ] `kh doctor` passes cleanly
- [ ] `kh workflow list` works
- [ ] MCP server connected (or curl test passes)
- [ ] Visual builder workflow created and run manually
- [ ] Agentic wallet installed + `~/.keeperhub/wallet.json` backed up
- [ ] `auto_approve_max_usd: 0.50` set in `safety.json`
- [ ] x402 paid call tested end-to-end with a small amount
- [ ] Rust toolchain (`cargo --version`) confirmed
- [ ] `cargo check` from the workspace root passes

## Done =
Every box in `plans/setup-verified.md` is ticked, the doc has a completion date, and the API key + wallet address are in 1Password.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,size-M" "$MILESTONE_PRE"

# ============ #2 ============
create_issue 2 "#2 — keeperhub-rs: implement list_workflows" "$(cat <<'EOF'
## Why
The agent's primary discovery mechanism. Without it, we can't find or call any workflow.

## Depends on
- #1

## What to do
- Replace the `Err(Error::Internal(...))` stub in `crates/keeperhub-rs/src/mcp.rs::list_workflows` with a real implementation
- POST a JSON-RPC `tools/call` request to the MCP endpoint
- Parse the response into `Vec<Workflow>`
- Add integration tests against the live MCP server (use a feature flag for the real test)

## Acceptance criteria
- [ ] `list_workflows().await` returns `Vec<Workflow>` populated from the live API
- [ ] Empty org returns empty vec (not error)
- [ ] Errors map cleanly to `Error::Api { status, message }` for 4xx/5xx
- [ ] Integration test: `cargo test --features live-mcp` passes against a real org
- [ ] `cargo run -p keeperhub-rs --example list_workflows` prints the workflow list

## Done =
The example prints real workflows from your org.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-rs,rust,size-M" "$MILESTONE_PRE"

# ============ #3 ============
create_issue 3 "#3 — keeperhub-rs: implement call_workflow (free)" "$(cat <<'EOF'
## Why
The agent's primary execution mechanism. We need to call a workflow and get a result.

## Depends on
- #2

## What to do
- Add `McpClient::call_workflow(slug, inputs).await -> Result<JsonValue>`
- Returns the workflow's output for free workflows
- Returns a `402` with a `PaymentChallenge` for paid workflows (actual payment in #6)

## Acceptance criteria
- [ ] Free workflow calls return parsed JSON output
- [ ] Paid workflow calls return `Err(Error::X402Unpaid(challenge))`
- [ ] Errors map cleanly to `Error::Api` for 4xx/5xx
- [ ] Integration test passes

## Done =
You can call a free KeeperHub workflow from Rust.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-rs,rust,size-M" "$MILESTONE_PRE"

# ============ #4 ============
create_issue 4 "#4 — keeperhub-rs: implement get_execution_logs" "$(cat <<'EOF'
## Why
The agent's primary observability. Every action must end up in the audit trail that we can query programmatically.

## Depends on
- #2

## What to do
- Add `McpClient::get_execution_logs(execution_id).await -> Result<ExecutionLogs>`
- Parse into the `ExecutionLogs` type already in `types.rs`

## Acceptance criteria
- [ ] Returns full structured logs (LogEntry per node)
- [ ] Empty logs handled
- [ ] Integration test passes

## Done =
A previously-executed workflow's logs can be retrieved in full.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-rs,rust,size-S" "$MILESTONE_PRE"

# ============ #5 ============
create_issue 5 "#5 — keeperhub-rs: x402 EIP-3009 builder" "$(cat <<'EOF'
## Why
The auto-pay mechanism. Without signing EIP-3009, the agent can't pay for paid KeeperHub workflows.

## Depends on
- #1

## What to do
- Implement `parse_challenge` in `x402.rs` (already has a stub)
- Implement `build_payment_header` (returns base64-encoded JSON of `{signature, authorization}`)
- Add a `PaymentSigner` trait that abstracts the signing backend
- Add a `KeeperHubProxySigner` that calls the KeeperHub signing endpoint (Turnkey-mediated) — avoids holding private keys locally
- Add a `LocalSigner` (with `alloy-rs`) as a fallback for self-custody path (skeleton only)

## Acceptance criteria
- [ ] `parse_challenge` correctly parses a real 402 body
- [ ] `build_payment_header` produces a valid x402 `X-PAYMENT` header
- [ ] `PaymentSigner` trait is well-defined
- [ ] `KeeperHubProxySigner` at minimum constructs the right request shape
- [ ] Unit tests for the typed-data hash

## Done =
A 402 challenge from a real paid workflow can be parsed and converted into a signed payment header.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-rs,rust,x402,size-M" "$MILESTONE_PRE"

# ============ #6 ============
create_issue 6 "#6 — keeperhub-rs: call_paid_workflow with x402 auto-pay" "$(cat <<'EOF'
## Why
This is the "paying customer" mechanism. The whole pitch.

## Depends on
- #3
- #5

## What to do
- Add `McpClient::call_paid_workflow(slug, inputs, signer).await -> Result<JsonValue>`
- On 402, parse the challenge, ask the signer for a payment, retry with the X-PAYMENT header
- On 200, parse the workflow result
- Surface all errors clearly

## Acceptance criteria
- [ ] Free workflow calls work (no payment attempted)
- [ ] Paid workflow calls successfully auto-pay and return the result
- [ ] Insufficient-balance error maps to `Error::X402Unpaid`
- [ ] Retry on 402 is automatic and idempotent
- [ ] Integration test on Sepolia testnet passes (small $0.01 call)

## Done =
A real paid workflow can be called from Rust with auto-pay working.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-rs,rust,x402,size-L" "$MILESTONE_PRE"

# ============ #7 ============
create_issue 7 "#7 — Build Aave V3 yield workflow in visual builder" "$(cat <<'EOF'
## Why
The yield strategy that funds the agent. Needs to be callable from Rust as a single slug.

## Depends on
- #1

## What to do
- Open the KeeperHub visual builder
- Create workflow `moltbot-aave-supply` with: Manual trigger, input: `asset` + `amount` (atomic units), action: `web3/write-contract` calling Aave V3 pool `supply(asset, amount, agentAddress, 0)`, returns tx hash
- Create a companion `moltbot-aave-withdraw` workflow
- Test both manually in the visual builder
- Note the slugs

## Acceptance criteria
- [ ] Supply workflow can be called manually with USDC, produces a real Aave tx
- [ ] Withdraw workflow works
- [ ] Both workflows are listed in `search_workflows`
- [ ] Both appear in the runs panel after testing

## Done =
Two working Aave workflows (supply, withdraw) callable by slug.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-workflow,size-M" "$MILESTONE_PRE"

# ============ #8 ============
create_issue 8 "#8 — Build Morpho health-check workflow in visual builder" "$(cat <<'EOF'
## Why
The job the agent does. Needs to be callable from Rust as a single slug.

## Depends on
- #1

## What to do
- Create workflow `moltbot-morpho-health` with: Manual trigger, input: `position_id`, action: read Morpho position + compute health factor, return as JSON
- Create a companion `moltbot-morpho-collateralize` workflow (if HF < threshold, supply more collateral)
- Test both manually on a real Morpho position (small test position on Base)

## Acceptance criteria
- [ ] Health-check returns structured JSON: `{ health_factor: float, collateral: string, debt: string }`
- [ ] Both workflows are listed in `search_workflows`
- [ ] Tested on a real Morpho position

## Done =
Two working Morpho workflows callable by slug.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,keeperhub-workflow,size-M" "$MILESTONE_PRE"

# ============ #9 ============
create_issue 9 "#9 — moltbot: agent loop skeleton (60s tick)" "$(cat <<'EOF'
## Why
The core loop. Without it, nothing else runs.

## Depends on
- #2

## What to do
- Replace placeholder `crates/moltbot/src/main.rs` with a real loop
- Add `crates/moltbot/src/config.rs` (AgentConfig loading from TOML + env)
- Add `crates/moltbot/src/loop.rs` (the `tick()` function, called every 60s)
- Add `crates/moltbot/src/state.rs` (in-memory state: balance, last action, safe mode)
- For now, the loop just logs "iteration N started" and the current USDC balance

## Acceptance criteria
- [ ] `cargo run -p moltbot` starts and runs forever
- [ ] Every 60s, logs an iteration line with timestamp + USDC balance
- [ ] Graceful shutdown on SIGINT
- [ ] Config loaded from `moltbot.toml`
- [ ] Missing config errors clearly

## Done =
Binary runs forever, logs every 60s, shuts down cleanly.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,moltbot-agent,rust,size-M" "$MILESTONE_PRE"

# ============ #10 ============
create_issue 10 "#10 — moltbot: yield strategy (call Aave, parse)" "$(cat <<'EOF'
## Why
The "self-funding" part. The agent decides when to park in Aave and calls the workflow.

## Depends on
- #6
- #7
- #9

## What to do
- Add `crates/moltbot/src/yield_strategy.rs`
- Decision: if USDC > park_threshold (default $50), call Aave supply; if USDC < withdraw_threshold (default $20), call Aave withdraw
- Parse the result, log the tx hash
- Update the in-memory state with the new balance

## Acceptance criteria
- [ ] On balance > $50, agent calls Aave supply, logs the tx hash
- [ ] On balance < $20, agent calls Aave withdraw, logs the tx hash
- [ ] On balance in between, no action
- [ ] Decision logic is unit-tested with mocked balances

## Done =
Agent can park and withdraw from Aave autonomously.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,moltbot-agent,rust,size-M" "$MILESTONE_PRE"

# ============ #11 ============
create_issue 11 "#11 — moltbot: job system + Morpho impl" "$(cat <<'EOF'
## Why
The job the agent does. Pluggable so we can add more jobs later.

## Depends on
- #6
- #8
- #9

## What to do
- Add `crates/moltbot/src/job.rs` (Job trait)
- Add `crates/moltbot/src/jobs/morpho_health.rs` (the Morpho job)
- The job calls `moltbot-morpho-health`, parses the result, decides whether to call `moltbot-morpho-collateralize`
- Add a `Job` enum and dispatcher

## Acceptance criteria
- [ ] Job trait is well-defined: `name()`, `tick()`, `should_run(state) -> bool`
- [ ] Morpho job runs on every tick
- [ ] Decision logic is unit-tested
- [ ] Adding a second job (e.g., `price_alert`) requires <50 lines

## Done =
Agent monitors a Morpho position and auto-collateralizes on low health.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,moltbot-agent,rust,size-L" "$MILESTONE_PRE"

# ============ #12 ============
create_issue 12 "#12 — moltbot: safe-mode (low-balance detection)" "$(cat <<'EOF'
## Why
Safety. If the agent runs out of USDC, it should stop trying to pay and alert.

## Depends on
- #9

## What to do
- Add `crates/moltbot/src/safe_mode.rs`
- On every tick, check if USDC balance < safe_mode_threshold (default $5)
- If yes, set `state.safe_mode = true`, send a Telegram alert (deferred to #23 if Telegram not yet integrated), and skip all paid actions
- If balance recovers above threshold, exit safe mode and alert

## Acceptance criteria
- [ ] On balance < $5, agent enters safe mode and logs a clear message
- [ ] Paid actions are skipped while in safe mode
- [ ] State is correctly restored when balance recovers
- [ ] Unit-tested

## Done =
Agent self-protects against running out of money.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,pre-build,moltbot-agent,rust,size-S" "$MILESTONE_PRE"

# ============ #13 ============
create_issue 13 "#13 — keeperhub-rs: implement search_workflows" "$(cat <<'EOF'
## Why
Discovery for the agent at runtime. The agent needs to find the right workflow by name, not just hard-code slugs.

## Depends on
- #2

## What to do
- Add `McpClient::search_workflows(query, category, tag).await -> Result<Vec<Workflow>>`
- Map to the `search_workflows` MCP tool

## Acceptance criteria
- [ ] Free-text query returns matching workflows
- [ ] Category filter works
- [ ] Tag filter works
- [ ] Integration test passes

## Done =
Agent can find workflows by name at runtime.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,pre-build,keeperhub-rs,rust,size-S" "$MILESTONE_PRE"

# ============ #14 ============
create_issue 14 "#14 — moltbot: SQLite audit log" "$(cat <<'EOF'
## Why
The dashboard needs a local data source. KeeperHub's runs panel is the source of truth, but a local cache makes the dashboard fast and offline-capable.

## Depends on
- #9

## What to do
- Add `crates/moltbot/src/audit.rs`
- Use `sqlx` with SQLite
- Schema: `runs(id, started_at, ended_at, status, kind)`, `actions(id, run_id, kind, tx_hash, x402_payment, ...)`, `x402_payments(id, action_id, amount, asset, chain, tx_hash)`
- On every loop iteration, write a run record
- On every action, write an action record
- On every x402 payment, write a payment record

## Acceptance criteria
- [ ] Schema migration runs cleanly on first start
- [ ] Every loop iteration persists a run record
- [ ] Every action persists an action record
- [ ] Every x402 payment persists a payment record
- [ ] All writes are wrapped in transactions (atomic per run)

## Done =
Local SQLite has a complete record of every agent action.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,pre-build,moltbot-agent,rust,size-M" "$MILESTONE_PRE"

# ============ #15 ============
create_issue 15 "#15 — moltbot: Axum dashboard rendering audit log" "$(cat <<'EOF'
## Why
The demo's visual centerpiece. Judges see the audit trail come alive.

## Depends on
- #14

## What to do
- Add `crates/moltbot/src/dashboard.rs`
- Axum server on `localhost:3030` (or configurable)
- Routes: `GET /` (HTML dashboard), `GET /api/runs` (JSON list of recent runs), `GET /api/runs/:id` (JSON single run detail), `GET /api/stats` (aggregate stats)
- Static `index.html` + `app.js` + `style.css` in `crates/moltbot/static/`
- The page polls `/api/runs` every 10s and renders a live view

## Acceptance criteria
- [ ] Dashboard loads at `http://localhost:3030`
- [ ] Shows recent runs with tx hashes (clickable to Etherscan)
- [ ] Shows aggregate stats: total earned, total spent, current balance
- [ ] Updates every 10s without page refresh
- [ ] Looks clean enough for a demo video (Tailwind or hand-rolled CSS)

## Done =
A live dashboard showing the agent's economic life.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,pre-build,dashboard,rust,size-L" "$MILESTONE_PRE"

# ============ #16 ============
create_issue 16 "#16 — Wire Aave yield into loop, first onchain tx" "$(cat <<'EOF'
## Why
The moment MoltBot moves real value onchain. The proof of execution.

## Depends on
- #10

## What to do
- Fund the agent's wallet with $50 USDC on Base mainnet (use a CEX or your own wallet)
- Run the agent
- Confirm: agent detects balance > threshold, calls Aave supply, USDC moves to Aave, balance drops accordingly
- Note the tx hash

## Acceptance criteria
- [ ] Agent makes a real Aave supply tx
- [ ] The tx hash is in the audit log
- [ ] The x402 payment for the keeper call is in the audit log
- [ ] Dashboard shows the run

## Done =
A real onchain tx, attributable to MoltBot, visible in the dashboard.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,moltbot-agent,rust,size-M" "$MILESTONE_BUILD"

# ============ #17 ============
create_issue 17 "#17 — Wire Morpho job into loop, real decision" "$(cat <<'EOF'
## Why
The moment the agent does useful work. Proof of agency.

## Depends on
- #11

## What to do
- Set up a small Morpho position with intentionally low health factor (e.g., supply $100 of wstETH, borrow $70 of USDC, then drop ETH price ~10%)
- Run the agent
- Confirm: agent detects HF < 1.3, calls Morpho collateralize, position health restored
- Note the tx hashes

## Acceptance criteria
- [ ] Agent detects low HF
- [ ] Agent calls the collateralize workflow
- [ ] The HF is restored above 1.3
- [ ] Tx hashes in audit log

## Done =
A real agent decision, executed onchain, visible in the dashboard.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,moltbot-agent,rust,size-M" "$MILESTONE_BUILD"

# ============ #18 ============
create_issue 18 "#18 — x402 end-to-end on Sepolia testnet" "$(cat <<'EOF'
## Why
Verify the x402 auto-pay works against a real chain before going to mainnet.

## Depends on
- #6

## What to do
- Find or create a paid workflow on Sepolia
- Run the agent against it
- Confirm: 402 returned, signed, paid, result returned
- Note the x402scan entry

## Acceptance criteria
- [ ] x402 payment settles on Sepolia
- [ ] x402scan shows the payment entry
- [ ] Workflow returns the expected result after payment
- [ ] Agent logs the payment

## Done =
A real x402 payment visible on x402scan.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,x402,size-M" "$MILESTONE_BUILD"

# ============ #19 ============
create_issue 19 "#19 — 24h stress test (20+ onchain txs)" "$(cat <<'EOF'
## Why
The "this thing actually works" demo. Judges need to see sustained, real execution.

## Depends on
- #16
- #17
- #18

## What to do
- Run the agent for 24+ hours on Base mainnet (or testnet if mainnet is too expensive)
- Let it loop normally — yield + job + audit
- At the end, snapshot the dashboard, save the SQLite DB
- Verify 20+ onchain txs, 100+ audit log entries, no crashes

## Acceptance criteria
- [ ] Agent ran for 24+ hours without crashing
- [ ] 20+ onchain txs in the audit log
- [ ] x402 payments visible on x402scan
- [ ] Dashboard renders all of it
- [ ] SQLite DB snapshot saved to `plans/24h-run-<date>.db`

## Done =
A clean 24h run with 20+ real txs.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,moltbot-agent,size-XL" "$MILESTONE_BUILD"

# ============ #20 ============
create_issue 20 "#20 — Record 90-second demo video" "$(cat <<'EOF'
## Why
The submission's most important asset. Judges watch this first.

## Depends on
- #19

## What to do
- Storyboard per `plans/moltbot-deep-research.md` §12.1 (problem, setup, loop, evidence, differentiator, call)
- Record the 5 segments
- Edit to 90s
- Upload as unlisted YouTube
- Embed in submission

## Acceptance criteria
- [ ] Video is 90-120s
- [ ] Shows a real onchain tx (Etherscan visible)
- [ ] Shows an x402 payment (x402scan or KeeperHub audit visible)
- [ ] Shows the dashboard with multiple runs
- [ ] Has a clear call-to-action (GitHub, dashboard link)
- [ ] Audio is clean (or muted with on-screen text)

## Done =
Unlisted YouTube link, embedded in the submission.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,dashboard,size-L" "$MILESTONE_BUILD"

# ============ #21 ============
create_issue 21 "#21 — Submission copy + README polish" "$(cat <<'EOF'
## Why
The submission is the first thing judges read after the video.

## Depends on
- #19

## What to do
- Write the 250-word submission copy per `plans/moltbot-deep-research.md` §12.2
- Polish the README (screenshots, GIF, link to video)
- Write `DEMO.md` with reproduction steps
- Write `ARCHITECTURE.md` (slim version of the research doc)
- Write `SCORING.md` mapping features to rubric
- Add `LICENSE` (MIT)
- Add `.env.example` (no secrets)

## Acceptance criteria
- [ ] Submission copy is 250 words, tells the story in 2-3 sentences
- [ ] README has 3+ screenshots above the fold
- [ ] DEMO.md has a one-command reproduction
- [ ] ARCHITECTURE.md has the agent loop diagram
- [ ] SCORING.md has the rubric-by-rubric breakdown
- [ ] Repo is publicly visible

## Done =
Everything judges need to understand the project is in the repo.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,dashboard,size-M" "$MILESTONE_BUILD"

# ============ #22 ============
create_issue 22 "#22 — Submit to DoraHacks (target: Aug 9)" "$(cat <<'EOF'
## Why
Deadline is Aug 13 12:00 UTC+2. Submitting Aug 9 gives 4 days of buffer.

## Depends on
- #20
- #21

## What to do
- Go to https://dorahacks.io/hackathon/agents-onchain/buidl
- Click "Submit BUIDL"
- Fill in: title, description (paste submission copy), GitHub link, demo video link, transaction link
- Required fields: GitHub link, demo video, link to a tx your agent executed

## Acceptance criteria
- [ ] Submission live on DoraHacks
- [ ] All three required fields filled
- [ ] Submission is publicly visible

## Done =
Submission live, before Aug 13 deadline.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,build,size-S" "$MILESTONE_BUILD"

# ============ #23 ============
create_issue 23 "#23 — Telegram alert integration" "$(cat <<'EOF'
## Why
Safe-mode alerts need a way to reach the operator. Telegram is the easiest.

## Depends on
- #12

## What to do
- Set up a Telegram bot (BotFather, 1 minute)
- Add the bot's chat_id to config
- On safe mode entry/exit, send a Telegram message via KeeperHub's Telegram plugin
- On every x402 payment, optionally send a small alert

## Acceptance criteria
- [ ] Safe mode entry triggers a Telegram message
- [ ] Safe mode exit triggers a Telegram message
- [ ] Messages are formatted cleanly

## Done =
Operator gets Telegram alerts on safe mode events.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,build,moltbot-agent,size-S" "$MILESTONE_BUILD"

# ============ #24 ============
create_issue 24 "#24 — Pre-x402 balance check + skip-when-low" "$(cat <<'EOF'
## Why
Defense in depth. Before every paid call, check balance and skip if too low.

## Depends on
- #6
- #12

## What to do
- Add a check in the loop: before calling `call_paid_workflow`, verify balance > max_x402_payment (configurable, default $0.10)
- If not, log and skip the call
- Don't enter safe mode (that's reserved for < $5 floor)

## Acceptance criteria
- [ ] Pre-call balance check in place
- [ ] Skip-when-low logs clearly
- [ ] Doesn't trip safe mode

## Done =
No accidental 402 prompts for high-cost calls.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,build,moltbot-agent,size-S" "$MILESTONE_BUILD"

# ============ #25 ============
create_issue 25 "#25 — keeperhub-rs: polish for crates.io publish" "$(cat <<'EOF'
## Why
The bounty deliverable needs to be a real, polished crate. Not a hackathon prototype.

## Depends on
- #6
- #13

## What to do
- Full doc comments on every public item
- `#![deny(missing_docs)]` clean
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- `cargo fmt` clean
- 80%+ test coverage on the MCP module
- At least one example that runs end-to-end
- Cargo.toml metadata complete: keywords, categories, description
- License file (MIT)
- README on the crate is the existing `keeperhub-rs/README.md`

## Acceptance criteria
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo doc --no-deps` produces clean docs
- [ ] `cargo package` succeeds
- [ ] Test coverage on mcp module is 80%+

## Done =
Crate is publishable.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,bounty,keeperhub-rs,rust,size-M" "$MILESTONE_BOUNTY"

# ============ #26 ============
create_issue 26 "#26 — keeperhub-rs: publish to crates.io" "$(cat <<'EOF'
## Why
The bounty explicitly rewards ecosystem contributions. Publishing to crates.io is the proof.

## Depends on
- #25

## What to do
- Get a crates.io API token (https://crates.io/me)
- `cargo login <token>`
- `cargo publish -p keeperhub-rs`
- Verify the crate is live

## Acceptance criteria
- [ ] Crate is live on https://crates.io/crates/keeperhub-rs
- [ ] README renders correctly on the crate page
- [ ] Docs build and are accessible

## Done =
Live crate on crates.io.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P0,bounty,keeperhub-rs,size-S" "$MILESTONE_BOUNTY"

# ============ #27 ============
create_issue 27 "#27 — Open PR to KeeperHub org (Rust adapter)" "$(cat <<'EOF'
## Why
The "merged PR" path of the bounty deliverable. Strong signal of community contribution.

## Depends on
- #26

## What to do
- Fork https://github.com/KeeperHub/keeperhub
- Add a `crates/keeperhub-rs` link or a "Rust SDK" section to their README
- Add a docs page for the Rust integration
- Open the PR with a clear description

## Acceptance criteria
- [ ] PR opened against KeeperHub/keeperhub
- [ ] PR description explains the integration, links to crates.io
- [ ] PR is reviewable (not WIP)

## Done =
Open PR on KeeperHub repo.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,bounty,keeperhub-rs,size-M" "$MILESTONE_BOUNTY"

# ============ #28 ============
create_issue 28 "#28 — Tutorial: 'Build your first Rust agent on KeeperHub'" "$(cat <<'EOF'
## Why
Onboarding improvements are explicitly part of the bounty. A tutorial is a strong contribution.

## Depends on
- #26

## What to do
- Write a `docs/tutorials/rust-agent.md` (or similar) for the KeeperHub docs site
- Walk through building a minimal Rust agent that calls a workflow
- Reference the keeperhub-rs crate
- Include the full source

## Acceptance criteria
- [ ] Tutorial is 500-1000 words
- [ ] Has a copy-pasteable code example
- [ ] Builds and runs end-to-end
- [ ] Submitted as a PR to KeeperHub docs

## Done =
Tutorial PR opened.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,bounty,keeperhub-rs,size-L" "$MILESTONE_BOUNTY"

# ============ #29 ============
create_issue 29 "#29 — Marketing: aggregated stats post" "$(cat <<'EOF'
## Why
"Look at this thing actually work" is the most viral content. Numbers > narrative.

## Depends on
- #19

## What to do
- Write a post: "MoltBot ran for 24h. It did X onchain txs, earned $Y in yield, spent $Z on x402, and never ran out of money."
- Include dashboard screenshots
- Post on X, /r/ethdev, /r/rust, KeeperHub Discord, Hacker News (Show HN)
- Tag KeeperHub, @daborahacks, key x402 accounts

## Acceptance criteria
- [ ] Post is live on at least 3 channels
- [ ] Includes the dashboard URL
- [ ] Includes a link to the GitHub repo

## Done =
Live posts on 3+ channels.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P1,post-submit,marketing,size-S" "$MILESTONE_POST"

# ============ #30 ============
create_issue 30 "#30 — Marketing: 'What I learned' thread" "$(cat <<'EOF'
## Why
Engagement post. Generates discussion, signals seriousness.

## Depends on
- #22

## What to do
- Write a thread: 5-7 things I learned building an autonomous onchain agent
- Include one specific gotcha (e.g., x402 ask-tier behavior, KeeperHub MCP quirks)
- End with a link to the repo

## Acceptance criteria
- [ ] Thread is 5-7 tweets long
- [ ] Has at least one specific, non-obvious lesson
- [ ] Ends with a CTA

## Done =
Thread posted on X.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P2,post-submit,marketing,size-S" "$MILESTONE_POST"

# ============ #31 ============
create_issue 31 "#31 — PR follow-up + community engagement" "$(cat <<'EOF'
## Why
Maintain momentum. Respond to issues, merge suggestions, post updates.

## Depends on
- #27

## What to do
- Watch for issues on the repo
- Respond to PRs
- Post weekly progress updates
- Engage in KeeperHub Discord

## Acceptance criteria
- [ ] No open issues older than 7 days without a response
- [ ] Weekly progress update posted

## Done =
Ongoing.

---
_Source: [plans/next-steps.md](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md)_
EOF
)" "P2,post-submit,marketing,size-S" "$MILESTONE_POST"

echo ""
echo "=== Summary ==="
echo "Created ${#URLS[@]} of 31 issues"
echo ""
for u in "${URLS[@]}"; do
  echo "  $u"
done
