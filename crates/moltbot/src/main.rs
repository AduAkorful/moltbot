//! MoltBot: the first paying customer of the KeeperHub marketplace.
//!
//! MoltBot is a Rust autonomous agent that:
//!
//! 1. Holds a USDC balance on Base
//! 2. Parks idle funds in Aave V3 via a KeeperHub yield workflow
//! 3. Pays for keeper work in real time via x402 when it needs to read
//!    onchain state, claim rewards, or move funds
//! 4. Logs every action through KeeperHub's audit trail
//!
//! # Status
//!
//! Pre-alpha. This is the binary entry point. The actual agent loop
//! lands in Phase 5 of the project plan.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    tracing::info!(
        name = "MoltBot",
        version = env!("CARGO_PKG_VERSION"),
        "starting up"
    );
    tracing::info!(
        "Pre-alpha scaffold. Real agent loop lands in Phase 5 of plans/moltbot-deep-research.md."
    );
    tracing::info!("See plans/setup-verified.md for the local environment checklist.");

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,moltbot=debug")),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
