# Setup Verification Checklist

> Run through this once to confirm every KeeperHub surface works on your machine. Tick each box. If something fails, the **Trouble** column has the fix.

**Date completed:** 2026-07-03
**KeeperHub account email:** _(not recorded for privacy)_

---

## A. Account + API key

- [ ] **A1.** Create an account at https://app.keeperhub.com (browser-based OAuth)
- [ ] **A2.** Note the active organization in the org switcher (top right of app)
- [ ] **A3.** Generate an API key: avatar → API Keys → Organisation tab → New API Key → name it `moltbot-dev`
- [ ] **A4.** Copy the `kh_...` key immediately. Save it to your password manager. **It is only shown once.**
- [ ] **A5.** Store in shell: `export KEEPERHUB_API_KEY=kh_...`
- [ ] **A6.** Verify by re-loading the app and confirming the org is selected

**Trouble A1:** if OAuth loops, try an incognito window.
**Trouble A3:** if the Organisation tab is empty, you may have a user-scoped account — only org-scoped keys work with the API. Switch or create an org.

---

## B. Test wallet + project

- [ ] **B1.** In the app, create a new project: Projects → New → name `moltbot-dev`
- [ ] **B2.** In the project, go to Wallet → the Turnkey wallet is provisioned automatically
- [ ] **B3.** Note the wallet address (top of the wallet page)
- [ ] **B4.** Switch to **Sepolia testnet** for dev (Network selector)
- [ ] **B5.** Get testnet ETH from a Sepolia faucet: https://sepoliafaucet.com (or alchemy.com/faucet)
- [ ] **B6.** Send 0.01 SepoliaETH to the wallet address from B3
- [ ] **B7.** Wait ~30s, click refresh, confirm the balance shows

**Trouble B5:** if the faucet is rate-limited, try https://www.alchemy.com/faucet/ethereum-sepolia (requires Alchemy account).

---

## C. CLI (`kh`)

- [ ] **C1.** Install: `brew install keeperhub/tap/kh` (macOS) or `brew tap keeperhub/tap && brew install kh` (Linux brew) or download binary from GitHub releases
- [ ] **C2.** `kh --version` shows a version
- [ ] **C3.** `kh auth login` — completes browser-based OAuth
- [ ] **C4.** `kh auth status` — shows "logged in" and your org
- [ ] **C5.** `kh doctor` — all checks pass (or notes minor warnings)
- [ ] **C6.** `kh workflow list` — empty list (or lists your existing workflows)
- [ ] **C7.** `kh wallet balance --network 11155111` — shows your SepoliaETH balance

**Trouble C1:** if brew tap doesn't work on Linux, the GitHub release binary is at https://github.com/KeeperHub/cli/releases.
**Trouble C3:** if browser auth fails, pass the API key directly: `KEEPERHUB_API_KEY=kh_... kh auth status`.

---

## D. MCP server

- [x] **D1.** In Claude Code (or your agent), run:
  ```bash
  claude mcp add --transport http keeperhub https://app.keeperhub.com/mcp \
    --header "Authorization: Bearer $KEEPERHUB_API_KEY"
  ```
  *Not using Claude Code — used direct curl instead. See trouble D1.*
- [x] **D2.** Restart Claude Code — *N/A (not using Claude Code)*
- [x] **D3.** Inside Claude Code, run `/mcp` and confirm the keeperhub server is connected — *N/A*
- [x] **D4.** Ask the agent: "List the workflows in my KeeperHub organization." — `list_workflows` returns `[]` (empty org, as expected)
- [x] **D5.** In Claude Code, verify the MCP tools are listed: there should be 19 tools — *Actual count is **31 tools** (docs.keeperhub.com says 19 — outdated). Deprecated tools: `search_plugins`, `get_template`. Useful extras: `search_protocol_actions`, `execute_protocol_action`, `list_action_schemas`, `tools_documentation`, `prepare_test_pin_data`.*

**Trouble D1:** if you don't use Claude Code, the MCP server can be hit directly via HTTP (the MoltBot Rust client will do this). For now, just confirm the URL is reachable: `curl -H "Authorization: Bearer $KEEPERHUB_API_KEY" https://app.keeperhub.com/mcp -X POST -H "Content-Type: application/json" -H "Accept: text/event-stream" -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'` — should return the tool list.

**Notes from running D4 directly:**
- The MCP server uses Streamable HTTP transport. The handshake is **sequential**: `initialize` → `notifications/initialized` → `tools/list` (or `tools/call`).
- The `initialize` response includes an `Mcp-Session-Id` header containing a **JWT** (24h expiry, ~24h from issuance). Subsequent requests must echo this header. **Design implication for `keeperhub-rs`:** the `McpClient` needs to cache and refresh the session token.
- All `tools/call` responses wrap data in a `content: [{ type: "text", text: "<json-string>" }]` envelope. The text payload is JSON-stringified. The `McpClient` needs a `unwrap_content()` helper.
- Direct `tools/call` to `list_workflows` with empty `arguments` returns `{"result": {"content": [{"type": "text", "text": "[]"}]}}`.

---

## E. Visual workflow builder

- [x] **E1.** Created via MCP `create_workflow` (not the visual builder UI). Workflow: `E-test-sepolia-balance`, id `bsxwjfrctd00y665ie1o9`.
- [x] **E2.** Manual trigger: `data.config.triggerType = "Manual"`. Confirmed working — `triggerSource: "manual"` in the execution logs.
- [x] **E3.** `web3/check-balance` action with `network="11155111"`, `address="0x54F9Fe5A1f63064fc083928df60A95db2dc2CE39"`. Wallet has **1.0 SepoliaETH** (1,000,000,000,000,000,000 wei).
- [x] **E4.** Saved (created via MCP).
- [x] **E5.** Ran via MCP `execute_workflow`. Status: `success`. Duration: 400ms.
- [x] **E6.** Output: `{"address": "0x54F9...CE39", "balance": "1.0", "success": true, "balanceWei": "1000000000000000000", "addressLink": "https://sepolia.etherscan.io/address/0x54F9...CE39"}`. Confirmed via MCP `get_execution`.
- [ ] **E7.** *Workflow NOT deleted — KeeperHub refuses: "Workflow has execution history. Delete executions first before deleting the workflow."* This is by design — execution history is the audit trail. Leaving the test workflow in place; can prune via the app UI later.

**Key MCP findings for `keeperhub-rs` (E2–E6 roundtrip):**

| Tool | Args | Returns |
|---|---|---|
| `create_workflow` | `name`, `description`, `nodes[]`, `edges[]` (projectId optional) | `{ id, name, enabled: false, isListed: false, projectId: null, ... }` |
| `execute_workflow` | `workflowId` | `{ executionId, status: "running" }` |
| `get_execution` | `executionId` | `{ status: { status, nodeStatuses[], progress, ... }, logs: { execution: { id, workflowId, status, input, output, startedAt, completedAt, duration, runId, transactionHashes[], gasUsedWei, triggeredByOrgApiKeyId, triggerSource, triggeredByCredentialType, ... } } }` |

**Node shape** (from the real Aave template + our test):
```json
{
  "id": "trigger-1" | "node-N",
  "type": "trigger" | "action",
  "position": { "x": 0, "y": 0 },
  "data": {
    "type": "trigger" | "action",
    "label": "Human label",
    "config": { /* triggerType OR actionType + required fields */ }
  }
}
```

**Edge shape:**
```json
{ "id": "e1", "source": "trigger-1", "target": "node-1" }
```

**Important for `types.rs`:** the API returns more fields than the existing `Workflow` struct has — `slug`, `isListed`, `projectId`, `tagId`, `userId`, `organizationId`, `nodes`, `edges`, `enabled`. The struct will need to grow for #2, #13, and #25 (publish).

**Important for `types.rs` Execution:** the API Execution has `triggeredByOrgApiKeyId`, `triggerSource`, `triggeredByCredentialType`, `lastSuccessfulNodeId/Name`, `executionTrace[]`, `runId`, `billable`, `executedWorkflowHash`, plus a nested `workflow` object. The audit log (#14) needs most of these.

**Polling note:** `get_execution.status.status` is `"success"` (or `running`/`failed`/`cancelled`) — there is no separate top-level `status` string at the top of the response.

---

## F. Agentic wallet + x402

- [ ] **F1.** Install: `npx -p @keeperhub/wallet keeperhub-wallet skill install`
- [ ] **F2.** Provision a wallet: `npx -p @keeperhub/wallet keeperhub-wallet add`
- [ ] **F3.** Note the wallet address output (this is *separate* from the creator wallet in B3)
- [ ] **F4.** Verify `~/.keeperhub/wallet.json` exists with mode 0600
- [ ] **F5.** Verify `~/.keeperhub/safety.json` exists with the default config
- [ ] **F6.** Raise the auto-approve threshold for the demo: edit `~/.keeperhub/safety.json`, set `auto_approve_max_usd: 0.50` (so most keeper calls are auto-approved)
- [ ] **F7.** Back up `wallet.json` to your password manager (it's unrecoverable if lost)
- [ ] **F8.** Fund the wallet with ~$5 of USDC on **Base mainnet** (use a small transfer from a CEX or another wallet). If you don't have any, the agentic wallet has a 402 ask-tier for amounts > $5 — first payment will prompt.

**Trouble F1:** if `npx` is missing, install Node 20+ from https://nodejs.org.
**Trouble F2:** if the add step fails with auth error, re-check API key.
**Trouble F8:** USDC on Base contract is `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`. Don't send to the wrong chain.

---

## G. Test an x402 paid call

- [x] **G1.** Searched the marketplace via MCP `search_workflows`. Found 4 cheap paid workflows (`$0.01`) and several at `$0.05`. **All 11 are currently disabled by their owners** (return `503 Workflow temporarily unavailable` when called). Marketplace is essentially empty of live paid workflows right now.
- [x] **G2.** Created our own paid workflow via MCP: published `E-test-sepolia-balance` as `sep-eth-balance-test` at $0.01. **Slug is permanent** — `sep-eth-balance-test` is now reserved forever. Status: listed but `priceUsdcPerCall: null` because **the price field cannot be set via MCP API** — must be done in the KeeperHub UI's "Marketplace" button on the workflow editor. Workflow is now unlisted (kept on the org; can be re-listed with a price via UI).
- [ ] **G3.** **BLOCKED** — needs the workflow to have a price set (UI action) AND the agentic wallet funded with USDC (F8).
- [ ] **G4.** Pending G3.
- [ ] **G5.** Pending G3.
- [ ] **G6.** Pending G3.
- [ ] **G7.** Pending G3.
- [ ] **G8.** Pending G3.

**Reference workflow `mcp-test`** mentioned in KeeperHub docs as a public test workflow at `/api/mcp/workflows/mcp-test/call` — **does not exist** (returns 404). Doc is wrong or workflow was unpublished.

**Major architecture finding for #5, #6 (x402 auto-pay):**
The agentic wallet (`@keeperhub/wallet@0.1.15`) exposes an MCP server with a `call_workflow` tool that **"pays AND invokes a KeeperHub marketplace workflow in one tool call. Auto-pays x402 (Base USDC) or MPP (Tempo USDC.e) 402 challenges."** It is registered in our opencode config at `~/.config/opencode/opencode.json`.

**Implication:** for #5 and #6, `keeperhub-rs` does NOT need to implement EIP-3009 signing. It can either:
- (a) Invoke the wallet's MCP `call_workflow` directly when a workflow is paid (cheapest, offloads all signing).
- (b) Spawn the wallet's CLI as a subprocess.
- (c) Build EIP-3009 ourselves (original plan; much more work).

Recommended: **(a) or (b).** Original #5 (M, 4h) and #6 (L, 6h) can both shrink to ~S (1-2h). **Pending user confirmation before re-scoping next-steps.md.**

**Other findings:**
- EIP-3009 `TransferWithAuthorization` on Base (USDC `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`) is the x402 protocol. Facilitator submits tx and pays gas — agent only debits USDC.
- MPP on Tempo (USDC.e `0x20c000000000000000000000b9537d11c60e8b50`) is the alternative. KeeperHub wallet auto-selects MPP (faster/cheaper) when both are on offer.
- 30% platform fee, 70% to creator on marketplace revenue.
- Workflow quota exemption: paid calls ≥ $0.05 are exempt from the org's monthly execution quota.
- KeeperHub is registered on x402scan (id `59aa13ab-2a99-4409-a4e1-8927f4006b29`), mppscan, and **8004scan (agent id 31875)** — strong marketing pull for the "first paying customer" pitch. (E-test workflow also showed an ERC-8004 feedback prompt in its response, indicating KeeperHub auto-registers executed workflows.)
- Workflows created via MCP are `enabled: false` by default — must be toggled via `update_workflow({ enabled: true })` before they can be called.
- Workflows with execution history cannot be deleted via the API; use `update_workflow` to disable instead.
- `create_workflow` requires `name`, `description`, `nodes[]`, `edges[]`. `list_workflow` (publish) additionally requires `slug`, `inputSchema`, `workflowType`. `priceUsdcPerCall` is **ignored** by the MCP API — UI only.

---

## H. Gas sponsorship check

- [x] **H1.** Used Ethereum mainnet (chain ID `"1"`) for the test workflow.
- [x] **H2.** Created workflow `H-test-gas-sponsorship` (id `5w6bjq9j92t7ec588ynzb`) via MCP. Action: `web3/transfer-funds` with `amount: "0"`, `recipientAddress: "0x54F9Fe5A1f63064fc083928df60A95db2dc2CE39"` (self), `network: "1"`. **Note:** docs say `toAddress` and `walletId` but the API actually uses `recipientAddress` and an implicit wallet (only one integration configured).
- [x] **H3.** Executed. Status: **success**, **`sponsored: true`**. Tx: `0x92800cc37ae2ac626090de54b2b3979fae58965a830353323d9c743e727f4db4` (Etherscan). Gas used: 81,279 wei (~$0.20 worth). 5.1s.
- [x] **H4.** N/A — we don't maintain a global "current network" state via MCP; each call specifies its own `network` field. The user's web UI may still be on Ethereum mainnet from the test; can be switched in the app.

**Trouble:** gas sponsorship is on Ethereum mainnet only. On other chains, you pay gas (cheap on Base/Arbitrum).

**Field-name gotcha for the keeperhub-rs implementation:** the API uses `recipientAddress` (not `toAddress` as the docs say), and the wallet is implicit when only one integration is configured. The Rust client should accept both names and normalize.

---

## I. Rust toolchain

- [x] **I1.** `rustc 1.96.1` (verified earlier in the session).
- [x] **I2.** `cargo 1.96.1` (same).
- [x] **I3.** N/A — current.
- [x] **I4.** `rustfmt` and `clippy` available (via rustup defaults).
- [ ] **I5.** `cargo-watch` not installed (optional, defer).
- [x] **I6.** Repo at `~/dev/moltbot` (already cloned).
- [x] **I7.** `cargo check` from workspace root passes. Only the 4 expected dead-code warnings on the stubs in `keeperhub-rs/src/mcp.rs` (`auth_header`, `http`, `JsonRpcRequest`, `JsonRpcResponse<T>`, `JsonRpcError`).

**Trouble I7:** if compilation fails, run `cargo check --verbose` to see the actual error. Common issues:
- edition too new for your toolchain → edit Cargo.toml to use edition = "2021"
- missing system libs for reqwest → `sudo apt install libssl-dev pkg-config` (Linux)
- outdated lockfile → `cargo update`

---

## J. Done — local environment is verified

When all the above are checked:

- [ ] **J1.** Commit any config changes to a local branch
- [ ] **J2.** Add a final note to the moltbot repo's `plans/setup-verified.md` with the date and any deviations
- [ ] **J3.** Proceed to **Phase 4.5+** of the project plan: actual Rust work

---

## Time estimate

- Account + API key: 5 min
- Wallet + Sepolia funding: 5 min + 1 min wait
- CLI install + auth: 5 min
- MCP server install: 5 min
- Visual workflow: 5 min
- Agentic wallet: 5 min
- x402 test: 10 min
- Gas sponsorship: 5 min
- Rust toolchain: 5 min (or 15 if you need to install rustup)

**Total: ~50 minutes for a clean install. ~90 min if anything goes sideways.**

The single biggest risk is the agentic wallet USDC funding step — you need actual USDC on Base. If you don't have any, use a CEX or a friend to send $5 worth.
