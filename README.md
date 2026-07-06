# MoltBot

An autonomous onchain agent that pays for its own operations through the
[KeeperHub](https://keeperhub.com) keeper economy. It calls onchain
workflows via x402, earns yield on idle USDC, and exposes every
economic decision through a local dashboard backed by KeeperHub's
audit trail.

MoltBot is built as a Rust workspace with two crates:

- **`moltbot`** — the agent binary. Ticks a state machine, decides
  whether to call workflows, executes yield strategies, and renders
  an Axum dashboard from a SQLite audit log.
- **`keeperhub-rs`** — a typed Rust client for the KeeperHub MCP
  server, with first-class helpers for Aave V3 and Morpho Blue plus
  pre-built marketplace workflow blueprints.

Originally developed for the [KeeperHub — Agents Onchain Hackathon](https://dorahacks.io/hackathon/agents-onchain/detail)
on DoraHacks.

## Features

- **x402-aware agent loop** — refuses paid calls when the wallet
  balance is below a per-call cap, distinct from safe-mode's $5
  floor.
- **Yield strategy** — supplies idle USDC to Aave V3 through
  KeeperHub's `execute_protocol_action` and reconciles state on
  success.
- **Safe mode** — enters a read-only state when total assets drop
  below a configurable floor; exits on recovery; emits Telegram
  alerts on every transition.
- **Morpho health job** — monitors a configured Morpho Blue
  position and tops up collateral when its health factor falls
  below the target.
- **SQLite audit log** — every tick, action, and x402 payment is
  persisted atomically per tick and surfaced through a local
  dashboard with Etherscan-linked transactions.
- **Typed KeeperHub client** — JSON-RPC over HTTP, JWT session
  handling, 402-challenge detection, and typed Aave V3 / Morpho
  Blue wrappers.

## Workspace layout

```
moltbot/
├── Cargo.toml                  workspace root
├── crates/
│   ├── keeperhub-rs/           Rust client for the KeeperHub MCP server
│   │   ├── src/{mcp,aave,morpho,workflows,x402,types,error,rest}.rs
│   │   ├── examples/           runnable examples (list, call, publish, ...)
│   │   ├── tests/              live-mcp integration tests
│   │   └── README.md           crate-level docs
│   └── moltbot/                the agent binary
│       ├── src/{main,config,state,tick,yield_strategy,safe_mode,
│       │        pre_x402,job,audit,dashboard,telegram}.rs
│       └── static/             dashboard assets
├── scripts/                    local development scripts
└── LICENSE
```

## Prerequisites

- Rust 1.75 or newer (`rustup default stable`)
- A KeeperHub API key (`KEEPERHUB_API_KEY`) for live calls
- A funded EVM wallet for onchain operations

## Build

```sh
git clone https://github.com/AduAkorful/moltbot.git
cd moltbot
cargo build --release
```

## Test

```sh
# unit tests (no network)
cargo test --workspace

# live MCP integration tests (gated, requires KEEPERHUB_API_KEY)
KEEPERHUB_API_KEY=kh_... cargo test --features live-mcp --test live_mcp -- --test-threads=1
```

The workspace has 213 unit tests and 4 doc tests. Live tests are
read-only.

## Lint

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

`rustfmt` is not part of the toolchain component set; install it with
`rustup component add rustfmt` if you want `cargo fmt --check`.

## Configuration

The agent reads `moltbot.toml` from the working directory, with
environment variables overriding specific fields. Key settings:

```toml
tick_interval_seconds = 60
network = "1"                               # Ethereum mainnet
wallet_address = "0xYOUR_WALLET_ADDRESS"
usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"

park_threshold_usd = 50.0                   # supply to Aave above this
withdraw_threshold_usd = 20.0               # pull back from Aave below this
safe_mode_threshold_usd = 5.0               # enter safe mode below this
max_x402_payment_usd = 0.10                 # per-call cap on paid workflows

dashboard_addr = "127.0.0.1:3030"
initial_usdc_balance_usd = 0.0              # seed state at startup

# optional
morpho_market_id = "0x..."                  # enables the Morpho health job
morpho_target_hf = 1.3
telegram_bot_token = "..."                  # or TELEGRAM_BOT_TOKEN env var
telegram_chat_id = "..."
```

## Run the agent

```sh
export KEEPERHUB_API_KEY=kh_...
cargo run --release -p moltbot
```

The dashboard is served at `http://127.0.0.1:3030` and reflects the
SQLite audit log live.

## Use the KeeperHub client

```rust,no_run
use keeperhub_rs::mcp::McpClient;
use keeperhub_rs::aave::AaveV3;
use keeperhub_rs::types::SearchWorkflowsOptions;

#[tokio::main]
async fn main() -> Result<(), keeperhub_rs::Error> {
    let client = McpClient::new("https://app.keeperhub.com/mcp", "kh_your_key");

    let defi = client.search_workflows(SearchWorkflowsOptions {
        category: Some("defi".into()),
        ..Default::default()
    }).await?;

    let _data = AaveV3::get_user_account_data(
        &client, "1", "0x0000000000000000000000000000000000000000"
    ).await?;

    Ok(())
}
```

See [`crates/keeperhub-rs/README.md`](crates/keeperhub-rs/README.md)
for the full module map and example list.

## License

MIT — see [LICENSE](LICENSE).
