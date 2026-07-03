# Setup Verification Checklist

> Run through this once to confirm every KeeperHub surface works on your machine. Tick each box. If something fails, the **Trouble** column has the fix.

**Date completed:** ___________
**KeeperHub account email:** ___________

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

- [ ] **G1.** Browse https://app.keeperhub.com/hub for a paid workflow (look for ones with a $ price per call)
- [ ] **G2.** If none exist yet, create one yourself:
  - In the app, create a new workflow: trigger=Manual, action=discord/webhook/something cheap
  - Publish to marketplace with a tiny price (e.g., $0.01)
- [ ] **G3.** In Claude Code (with the MCP server + agentic wallet both installed), call the paid workflow via the agent
- [ ] **G4.** Confirm the agent's logs show a 402 challenge intercepted, signed, and retried
- [ ] **G5.** Check the KeeperHub run logs — status=completed
- [ ] **G6.** Check your USDC balance on Base — debited by the call amount
- [ ] **G7.** Check the creator wallet — credited by the call amount
- [ ] **G8.** (Optional) Check x402scan.com for the settlement entry

**Trouble G1:** if no paid workflows exist, just use your own test workflow from G2.

---

## H. Gas sponsorship check

- [ ] **H1.** In the app, switch to **Ethereum mainnet** network
- [ ] **H2.** Create a workflow: trigger=Manual, action=web3/transfer-funds, to=some address, amount=0 (just to test)
- [ ] **H3.** Run it — confirm the tx was sponsored (your wallet wasn't debited gas)
- [ ] **H4.** Switch back to Sepolia or Base for continued dev

**Trouble:** gas sponsorship is on Ethereum mainnet only. On other chains, you pay gas (cheap on Base/Arbitrum).

---

## I. Rust toolchain

- [ ] **I1.** `rustc --version` — 1.75 or newer
- [ ] **I2.** `cargo --version` — same
- [ ] **I3.** If outdated: `rustup update stable`
- [ ] **I4.** (Optional) Install additional components: `rustup component add rustfmt clippy`
- [ ] **I5.** `cargo install cargo-watch` (optional but useful for `cargo watch -x run`)
- [ ] **I6.** Clone the moltbot repo: `cd ~/dev && git clone <url> moltbot` (or `cd moltbot` if already there)
- [ ] **I7.** `cargo check` from the workspace root — should compile cleanly with no errors

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
