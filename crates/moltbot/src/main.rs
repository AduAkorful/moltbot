//! MoltBot — the first paying customer of the KeeperHub marketplace.
//!
//! A Rust autonomous agent that funds itself via x402 and uses
//! KeeperHub's audit trail as its primary observability.
//!
//! This binary is the agent's runtime. It loads a TOML config, opens
//! an MCP client, and runs a periodic loop until interrupted. Yield
//! strategy, Morpho job, safe mode, and the audit dashboard are
//! layered on in subsequent iterations.
//!
//! # Usage
//!
//! ```sh
//! # Set the KeeperHub API key (required).
//! export KEEPERHUB_API_KEY=kh_...
//!
//! # Optional: point at a custom config file.
//! export MOLTBOT_CONFIG=/path/to/moltbot.toml
//!
//! # Run.
//! cargo run -p moltbot
//! ```
//!
//! # Modules
//!
//! - [`config`] — agent configuration loaded from TOML + env
//! - [`state`] — in-memory agent state (USDC balance, iteration, safe mode)
//! - [`tick`] — the main tick loop with SIGINT shutdown
//! - [`yield_strategy`] — Aave V3 supply/withdraw decision + execution
//! - [`job`] — the [`job::Job`] trait + [`job::JobRegistry`] dispatcher
//! - [`jobs`] — built-in [`job::Job`] implementations
//! - [`safe_mode`] — low-balance detection; skips paid actions
//!
//! Public re-exports in the crate root make the structure available
//! to integration tests under `tests/`.

pub mod config;
pub mod job;
pub mod jobs;
pub mod safe_mode;
pub mod state;
pub mod tick;
pub mod yield_strategy;

use std::path::PathBuf;
use std::sync::Arc;

use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use tracing_subscriber::EnvFilter;

use crate::config::AgentConfig;
use crate::job::JobRegistry;
use crate::jobs::morpho_health::MorphoHealthJob;
use crate::state::new_shared_state;
use crate::tick::AgentLoop;

/// CLI argument shape. Kept tiny — the bulk of the config is TOML + env.
#[derive(Debug, Default)]
struct Cli {
    /// Optional path to a TOML config file. Overrides the
    /// `MOLTBOT_CONFIG` env var.
    config: Option<PathBuf>,
}

fn parse_cli() -> Cli {
    let mut cli = Cli::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--version" => {
                println!("moltbot {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--config" => {
                cli.config = args.next().map(PathBuf::from);
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }
    cli
}

fn print_help() {
    println!(
        "moltbot {version}

USAGE:
    moltbot [--config <path>]

ENV:
    KEEPERHUB_API_KEY    KeeperHub API key (Bearer). Required.
    MOLTBOT_CONFIG       Path to a TOML config file. Optional.
    RUST_LOG             Standard tracing-subscriber filter, e.g.
                         'info', 'moltbot=debug'. Default: 'info'.

OPTIONS:
    --config <path>      Override the config file path.
    -h, --help           Print this help message and exit.
    --version            Print the version and exit.

CONFIG FILE:
    See `moltbot::config::AgentConfig` for the full schema. Example:

    tick_interval_seconds = 60
    network = \"1\"
    park_threshold_usd = 50.0
    withdraw_threshold_usd = 20.0
    safe_mode_threshold_usd = 5.0
    # wallet_address = \"0x...\"  # defaults to the org creator wallet
    # usdc_address   = \"0x...\"  # defaults to USDC on Ethereum mainnet
    # morpho_market_id = \"0x...\"  # enables the Morpho health-factor job
    # morpho_target_hf = 1.3
",
        version = env!("CARGO_PKG_VERSION"),
    );
}

fn resolve_config_path(cli: &Cli) -> Option<PathBuf> {
    cli.config
        .clone()
        .or_else(|| std::env::var("MOLTBOT_CONFIG").ok().map(PathBuf::from))
}

/// Set up tracing-subscriber from the `RUST_LOG` env var, defaulting
/// to `info` if unset or unparseable.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,moltbot=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cli = parse_cli();
    let config_path = resolve_config_path(&cli);
    let config = AgentConfig::from_env_and_file(config_path.as_deref())
        .map_err(|e| anyhow::anyhow!("config error: {e}"))?;
    let config = Arc::new(config);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        tick_seconds = config.tick_interval_seconds,
        network = %config.network,
        "moltbot starting"
    );

    // Construct the MCP client. `api_key` is `Some` after validation.
    let api_key = config
        .keeperhub_api_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("KEEPERHUB_API_KEY missing after validation"))?;
    let client = Arc::new(McpClient::new(DEFAULT_MCP_URL, api_key));

    // Eagerly initialize the session so a bad API key fails fast.
    if let Err(e) = client.initialize().await {
        tracing::error!(error = %e, "MCP initialize failed; check KEEPERHUB_API_KEY");
        return Err(anyhow::anyhow!("MCP initialize failed: {e}"));
    }
    tracing::info!(url = %client.url(), "MCP session established");

    let state = new_shared_state();

    // Build the job registry. Add new jobs here with
    // `JobRegistry::with(MyJob::new())`. A second job (e.g.
    // `PriceAlertJob`) is a ~30-line `impl Job` block + one line
    // here; the loop dispatcher is unchanged.
    let jobs = JobRegistry::new().with(MorphoHealthJob::new());
    tracing::info!(
        job_count = jobs.len(),
        "job registry built"
    );

    let loop_ = AgentLoop::new(state.clone(), client, Arc::clone(&config), jobs);
    let _shutdown = loop_.shutdown_handle();

    let iterations = loop_.run().await;
    tracing::info!(iterations, "moltbot exited cleanly");
    Ok(())
}
