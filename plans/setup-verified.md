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

- [ ] **D1.** In Claude Code (or your agent), run:
  ```bash
  claude mcp add --transport http keeperhub https://app.keeperhub.com/mcp \
    --header "Authorization: Bearer $KEEPERHUB_API_KEY"
  ```
- [ ] **D2.** Restart Claude Code
- [ ] **D3.** Inside Claude Code, run `/mcp` and confirm the keeperhub server is connected
- [ ] **D4.** Ask the agent: "List the workflows in my KeeperHub organization." — it should call `list_workflows` and return the list (probably empty)
- [ ] **D5.** In Claude Code, verify the MCP tools are listed: there should be 19 tools

**Trouble D1:** if you don't use Claude Code, the MCP server can be hit directly via HTTP (the MoltBot Rust client will do this). For now, just confirm the URL is reachable: `curl -H "Authorization: Bearer $KEEPERHUB_API_KEY" https://app.keeperhub.com/mcp -X POST -H "Content-Type: application/json" -H "Accept: text/event-stream" -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'` — should return the tool list.

---

## E. Visual workflow builder

- [ ] **E1.** In the app, create a new workflow: Workflows → New → blank workflow
- [ ] **E2.** Add a **Manual trigger** (default)
- [ ] **E3.** Add a **web3/check-balance** action: network=11155111, address=your wallet from B3
- [ ] **E4.** Save the workflow
- [ ] **E5.** Click **Run** — execution completes, status=completed
- [ ] **E6.** Open the run — confirm the output shows your SepoliaETH balance

This proves the visual builder works. You can delete this workflow after.

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
