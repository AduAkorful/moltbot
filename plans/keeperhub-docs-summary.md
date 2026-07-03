# KeeperHub Docs — Quick Reference

> Distilled from docs.keeperhub.com. Reference card for MoltBot development.

---

## 1. What KeeperHub actually is

A **no-code onchain automation platform** with three layers:
1. **Visual workflow builder** — drag-drop nodes (triggers + actions + conditions)
2. **REST + MCP + CLI** — programmatic surface for agents
3. **Marketplace** — publish a workflow, agents call it, pay per execution in USDC

The marketing page is misleadingly small. The *actual* platform is a Zapier-for-blockchain with 20+ DeFi protocol plugins and a paid workflow marketplace.

**The flywheel:** workflow author publishes a paid workflow → agent calls it via MCP/REST → agent pays USDC via x402 → author earns → author publishes more.

**MoltBot is the first *customer* in this flywheel.**

---

## 2. Core concepts

| Term | Definition |
|---|---|
| **Workflow** | A directed graph of nodes. Each workflow has triggers, actions, and conditions. |
| **Node** | A single step. Three types: trigger, action, condition. |
| **Trigger** | Starts execution: manual, schedule (cron), webhook, blockchain event, block interval. |
| **Action** | Performs work: web3, notification, system, math. |
| **Condition** | If/else branching. Operators: `==`, `===`, `!=`, `>`, `contains`, `matchesRegex`, `isEmpty`, etc. |
| **Execution** | One run of a workflow. Has a status (pending, running, completed, failed). |
| **Run logs** | Full audit trail: trigger, simulation result, submitted tx, gas used, outcome, timestamp. |
| **Wallet integration** | A Turnkey-backed EVM wallet that signs transactions for the workflow. |

Data flows between nodes via template syntax: `{{@nodeId:Label.field}}`.

---

## 3. Trigger types

| Type | When it fires | Use case |
|---|---|---|
| **Manual** | You click "Run" in builder | Testing |
| **Schedule** | Cron expression | Recurring automation |
| **Webhook** | External HTTP POST | External integration |
| **Blockchain Event** | Contract event emitted onchain | Event-driven |
| **Block Interval** | Every N blocks | Periodic onchain checks |

For MoltBot: we trigger workflows **manually** (from our Rust code via `call_workflow`). We're not building workflows; we're *calling* them.

---

## 4. Action types

### 4.1 Web3 (no wallet)
- `web3/check-balance` — ETH balance of address
- `web3/check-token-balance` — ERC-20 balance
- `web3/read-contract` — call `view`/`pure` function

### 4.2 Web3 (requires wallet)
- `web3/transfer-funds` — send ETH
- `web3/transfer-token` — send ERC-20
- `web3/write-contract` — call state-changing function

### 4.3 Notifications
- Discord, Telegram, SendGrid email

### 4.4 System
- HTTP requests, conditionals, loops, math, template rendering

### 4.5 Plugins (protocol-specific)
Aave V3, Compound V3, Uniswap, Morpho, Curve, Yearn V3, Lido, Pendle, Spark, Sky, Aerodrome, Ajna, CoW Swap, Rocket Pool, Frax Ether V2, Hyperliquid, Chainlink, Blockscout.

**For MoltBot's yield loop:** we'll use the **Aave V3 plugin** (supply/withdraw USDC). The plugin already handles contract calls, gas, retries.

---

## 5. MCP Server (the part MoltBot uses most)

**Endpoint:** `https://app.keeperhub.com/mcp` (remote, recommended) or local via `kh serve --mcp` (deprecated).

**Install for Claude Code:**
```bash
claude mcp add --transport http keeperhub https://app.keeperhub.com/mcp
```
Then `/mcp` to complete OAuth. Or pass API key directly:
```bash
claude mcp add --transport http keeperhub https://app.keeperhub.com/mcp \
  --header "Authorization: Bearer kh_your_key_here"
```

### 5.1 Tools (31 total as of v1.2.0; docs say 19 — outdated)

**Workflow management:**
- `list_workflows` — list org workflows (paginated, with `projectId` filter)
- `get_workflow` — get full config (nodes, edges) by ID
- `create_workflow` — create from explicit nodes/edges
- `update_workflow` — modify existing
- `delete_workflow` — remove
- `validate_workflow` — pre-flight structural/Web3 validation
- `prepare_test_pin_data` — return JSON Schema per node, for agent test data

**Execution:**
- `execute_workflow` — manually trigger, returns execution ID
- `get_execution` — combined status + step logs (replaces `get_execution_status` + `get_execution_logs`)

**AI generation:**
- `ai_generate_workflow` — natural-language workflow creation

**Discovery:**
- `list_action_schemas` — list action types by category
- `search_plugins` ⚠️ *DEPRECATED in v1.13 — use `list_action_schemas`*
- `get_plugin` — full plugin docs
- `search_templates` — pre-built workflows
- `get_template` ⚠️ *DEPRECATED in v1.13 — use `get_workflow`*
- `deploy_template` — deploy to your account

**Integrations:**
- `list_integrations` — configured integrations
- `get_wallet_integration` — wallet ID for write ops

**Direct DeFi (NEW — important for MoltBot):**
- `search_protocol_actions` — list supported DeFi actions across protocols (Aave, Morpho, etc.)
- `execute_protocol_action` — execute a DeFi action directly (e.g. Aave supply/withdraw, Morpho borrow/repay) without building a workflow
- `execute_transfer` — transfer native or ERC20
- `execute_contract_call` — call any contract function (read or write)
- `execute_check_and_execute` — read → evaluate condition → execute
- `get_direct_execution_status` — status of a direct exec

**Marketplace (the paying-customer path):**
- `search_workflows` — discover listed workflows
- `call_workflow` — invoke a listed workflow (returns 402 for paid)
- `list_workflow` — publish your workflow to the catalog
- `unlist_workflow` — remove from catalog
- `update_workflow_listing` — edit listing metadata
- `get_workflow_listing` — read listing metadata by slug

**Documentation:**
- `tools_documentation` — tool docs

**Resources (read-only):**
- `keeperhub://workflows` — list
- `keeperhub://workflows/{id}` — full config

**Architectural note for `keeperhub-rs` (discovered during D4):**
The MCP server uses **Streamable HTTP** transport. The `initialize` call returns an `Mcp-Session-Id` header (a **JWT, 24h expiry**); subsequent calls must echo it. `tools/call` responses wrap data in a `content: [{ type: "text", text: "<json-stringified>" }]` envelope. The `McpClient` needs a session cache + an `unwrap_content()` helper.

**Architectural note for `keeperhub-rs` (discovered during E2–E6):**
A workflow = `nodes[]` (each `{id, type, position, data: {type, label, config}}`) + `edges[]` (each `{id, source, target}`). Trigger nodes have `data.config.triggerType` (e.g. `Manual`, `Schedule`). Action nodes have `data.config.actionType` (e.g. `web3/check-balance`) + the required fields inline (no nested `inputs` object).

`execute_workflow` returns `{executionId, status: "running"}` — always running at first; poll `get_execution` until terminal. The response shape:
```json
{
  "status": { "status": "success|running|failed|cancelled", "nodeStatuses": [...], "progress": {...}, "transactionHashes": [] },
  "logs": { "execution": { "id", "workflowId", "status", "input", "output", "startedAt", "completedAt", "duration", "runId", "transactionHashes", "gasUsedWei", "triggeredByOrgApiKeyId", "triggerSource", "triggeredByCredentialType", "lastSuccessfulNodeId", "lastSuccessfulNodeName", "executionTrace", "billable", "executedWorkflowHash", "workflow": {...full embedded workflow...} } }
}
```

Workflows with execution history cannot be deleted via the API ("Delete executions first") — this is by design (audit trail is non-erasable). New workflows are `enabled: false` and `isListed: false` by default.

**Strategic simplification for #7/#8:** `execute_protocol_action` can replace building the Aave/Morpho workflows in the visual builder for our own yield strategy. Visual-builder workflows are still needed for #25-28 (bounty publish) and for any workflow we want to *charge* others to call. **Worth discussing at #7.**

### 5.2 Per-workflow MCP servers

Every listed marketplace workflow gets its own MCP server at `/mcp/w/<slug>`. The agent sees a *typed tool* (single decision step) instead of a generic `call_workflow(slug, inputs)` dispatcher. **Important for tool-picking accuracy.**

### 5.3 Paid workflows & x402

When a paid workflow is called, it returns HTTP 402 with an x402 challenge body. The MCP transport surfaces this as a tool error. To auto-pay, install the **agentic wallet** (see §7) — its PreToolUse safety hook intercepts the 402, signs the payment, retries.

### 5.4 Network field format

`"network"` accepts chain IDs as strings:
- `"1"` — Ethereum mainnet
- `"11155111"` — Sepolia
- `"8453"` — Base
- `"42161"` — Arbitrum
- `"137"` — Polygon

### 5.5 Error format

```json
{ "content": [{ "type": "text", "text": "Error: <message>" }], "isError": true }
```
Codes: 400 (bad params), 401 (no auth), 404 (not found), 500 (server).

---

## 6. REST API

**Base URL:** `https://app.keeperhub.com/api/v1` (verify in docs)
**Auth:** Bearer `kh_` API key, org-scoped.

**Endpoints MoltBot may use directly:**
- `GET /workflows` — list workflows
- `POST /workflows` — create
- `POST /workflows/{id}/execute` — trigger
- `GET /executions/{id}` — status
- `GET /executions/{id}/logs` — logs
- `GET /analytics` — usage stats
- `GET /chains` — supported chains
- `POST /direct-execution` — execute without saving a workflow

The REST API is for cases where MCP is overkill. For MoltBot's agent loop, MCP is the right choice.

---

## 7. Agentic wallet (x402 auto-pay)

**Custody model:** server-side Turnkey sub-org. Agent holds HMAC secret in `~/.keeperhub/wallet.json` (mode 0600). **No private key on disk.** Signing happens in Turnkey's secure enclave.

**Install:**
```bash
npx -p @keeperhub/wallet keeperhub-wallet skill install
npx -p @keeperhub/wallet keeperhub-wallet add
```

**Back up** `~/.keeperhub/wallet.json` like an SSH key. No recovery today.

### 7.1 Safety tiers (PreToolUse hook)

Reads from `~/.keeperhub/safety.json` (defaults shown):

| Tier | Behavior | Default |
|---|---|---|
| **auto** | Signs without prompt if amount ≤ threshold | ≤ $5 USD |
| **ask** | Inline permission prompt if amount ≤ threshold | $5 < x ≤ $100 |
| **block** | Denied | > $100 |

**For MoltBot: we want `auto_approve_max_usd: 0.50` or higher** so most keeper calls don't prompt us.

### 7.2 Server-side hard limits (Turnkey-enforced, not user-configurable)

- **Contract allowlist:** Base USDC (`0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`) and Tempo USDC.e (`0x20C000000000000000000000B9537D11c60E8b50`) only.
- **Per-transfer cap:** 100 USDC.
- **Approval cap:** 100 USDC.
- **Chain allowlist:** Base (8453), Tempo mainnet (4217), Tempo testnet (42431).
- **Daily spend cap:** 200 USDC per UTC day. `429 DAILY_CAP_EXCEEDED` if exceeded.

**Implication for MoltBot:** the demo runs for 24h and spends maybe $1-5. We're nowhere near the cap. But for "real" long-running agents, this is a meaningful ceiling.

### 7.3 x402 mechanics

**On Base:** sign EIP-3009 `TransferWithAuthorization` (pre-signed). Facilitator submits the on-chain tx and pays gas. Your wallet only debits USDC.

**On Tempo:** sign MPP authorization. Facilitator pays Tempo fees.

**Net:** a $0.05 paid workflow costs $0.05 USDC. No gas. No ETH.

### 7.4 What the agent sees

- `search_workflows` — find by category/tag/free text
- `call_workflow(slug, inputs)` — invoke

These are the same MCP tools; the wallet hooks into the 402 handling. **MoltBot's Rust client needs to implement the equivalent of the 402 auto-pay logic** (or use the agentic wallet's npm package — but that means running Node, which is ugly in a Rust binary).

### 7.5 Alternatives to the agentic wallet

| Option | Custody | Notes |
|---|---|---|
| **KeeperHub agentic wallet** | Server-side Turnkey | Default, recommended |
| **agentcash** | Plaintext key on disk | Testing only — key in `~/.agentcash/wallet.json` unencrypted |
| **Coinbase agentic wallet skills** | CDP-managed or self-custody | Brings CDP platform lock-in |

For MoltBot, we use the KeeperHub agentic wallet. We may also build a self-custody path for the bounty.

### 7.6 Known limitations

- Solana, Arbitrum, Optimism not yet supported (Base + Tempo only for x402)
- Browser-based review for large payments is on the roadmap
- Third-party x402 service discovery from the agent is on the roadmap

---

## 8. CLI (`kh`)

**Install:** `brew install keeperhub/tap/kh` (Homebrew) or download binary.

**Key commands:**

| Command | Purpose |
|---|---|
| `kh auth login` | Authenticate via browser |
| `kh auth status` | Check auth state |
| `kh doctor` | Verify environment |
| `kh workflow list` | List workflows |
| `kh workflow get <id>` | Get workflow config |
| `kh workflow run <id>` | Execute |
| `kh execute contract-call` | Direct contract call |
| `kh execute transfer` | Direct transfer |
| `kh execute status <id>` | Check status |
| `kh wallet balance` | Check wallet |
| `kh template list` | List marketplace templates |
| `kh template deploy <id>` | Deploy template |

---

## 9. Supported chains

- **Mainnet:** Ethereum, Base, Arbitrum, Polygon
- **Testnet:** Sepolia (and equivalents for others)
- **x402 signing chains:** Base (8453), Tempo mainnet (4217), Tempo testnet (42431) — only

For MoltBot: **Base mainnet** (cheapest, most active for x402). Use Sepolia for testing.

---

## 10. The Marketplace flywheel (the key insight)

**Marketplace mechanics:**
1. Author builds a workflow (e.g., "Aave V3 supply USDC").
2. Author publishes it to the marketplace, optionally with a price (e.g., $0.05/call).
3. Workflow appears in `search_workflows` results for everyone.
4. Agents (or humans) call it via MCP/REST.
5. Author's creator wallet receives USDC for each call.

**Two ways to interact with the marketplace:**

| Direction | Who | What |
|---|---|---|
| **Supplier** | Workflow author | Build, publish, earn USDC per call |
| **Customer** | Agent (MoltBot) | Discover, call, pay USDC |

**MoltBot is a customer.** It will:
- Call a yield workflow (e.g., Aave supply) to park idle USDC
- Call a job workflow (e.g., Morpho health check) to do its work
- Call paid data feeds (e.g., price oracles) to inform decisions

Every call is auditable on x402scan.com. Every payment is settled on Base. Every workflow call appears in KeeperHub's run logs.

---

## 11. Run lifecycle & audit trail

When a workflow runs:
1. **Trigger fires** (manual, schedule, etc.)
2. **Each node executes** in topological order
3. **Conditions branch** the flow
4. **Web3 actions** simulate first, then submit
5. **Tx is signed** by the wallet integration
6. **Retries** happen automatically with exponential backoff
7. **Status updates:** pending → running → completed/failed
8. **Logs capture:** trigger, each node's output, tx hashes, gas used, errors

**For MoltBot's observability:** `get_execution_logs(execution_id)` returns the full record. The dashboard queries this for the live view.

---

## 12. Hackathon-relevant gotchas

1. **Test on Sepolia first.** Don't waste mainnet ETH/USDC during dev.
2. **MCP endpoint is the public one** (`https://app.keeperhub.com/mcp`). No local process needed.
3. **Per-workflow MCP servers** at `/mcp/w/<slug>` are typed and LLM-friendly. Prefer them over the aggregate `call_workflow(slug, inputs)` for agent accuracy.
4. **Paid workflows return 402 with a challenge body.** Your client must handle this and sign EIP-3009 (or call the agentic wallet hook).
5. **`walletId` is required for write actions.** Get it via `get_wallet_integration` first.
6. **Network IDs are strings, not numbers.** `"8453"` not `8453u64`.
7. **The agentic wallet is custodial.** If the user wants self-custody, build it yourself with alloy-rs.
8. **The $200/day cap is real.** For 24h demos, fine. For production, raise with KeeperHub support.
9. **Gas sponsorship is on mainnet Ethereum only.** On Base/Arbitrum, you pay gas (but it's cheap).
10. **Templates at `kh template list` and via MCP `search_templates`** are good starting points — don't reinvent.

---

## 13. The skill files (for the bounty)

- `keeperhub/agentic-wallet-skills` — agentic wallet skill distribution
- `KeeperHub/eve-plugin` — Vercel Eve integration (TS)
- `KeeperHub/hermes-plugin` — Hermes integration (Python)
- `KeeperHub/mcp` — shared MCP client foundation (TS + Python)
- `KeeperHub/sdk` — official REST SDK (TS only)
- `KeeperHub/cli` — Go CLI
- `KeeperHub/agentic-wallet` — npm package

**Gap:** no Rust. **MoltBot fills this with `keeperhub-rs`.**

---

## 14. URLs cheat sheet

| Resource | URL |
|---|---|
| Main docs | https://docs.keeperhub.com |
| MCP server docs | https://docs.keeperhub.com/ai-tools/mcp-server |
| Agentic wallet docs | https://docs.keeperhub.com/ai-tools/agentic-wallet |
| CLI docs | https://docs.keeperhub.com/cli |
| App | https://app.keeperhub.com |
| Marketplace | https://app.keeperhub.com/hub |
| GitHub org | https://github.com/KeeperHub |
| Main repo | https://github.com/KeeperHub/keeperhub |
| Discord | https://discord.gg/keeperhub |
| x402 explorer | https://x402scan.com |
| x402 spec | https://docs.cdp.coinbase.com/x402 |
| Hackathon page | https://dorahacks.io/hackathon/agents-onchain/detail |
