//! Example: read a Morpho Blue position and compute its health factor.
//!
//! Demonstrates [`keeperhub_rs::morpho::Morpho`] typed helpers
//! (read-only, no wallet needed) and the pure-Rust
//! [`keeperhub_rs::morpho::compute_health_factor`] function.
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! # A real Morpho Blue market id (keccak256 of MarketParams), 0x-prefixed
//! export MORPHO_MARKET_ID=0x...
//! export MORPHO_NETWORK=1            # 1 = Ethereum, 8453 = Base
//! export MORPHO_USER=0xYourAddress
//! cargo run --example morpho_position
//! ```
//!
//! For demo, you can use the zero-bytes32 id to confirm the call
//! shape works end-to-end (the server will return an error for an
//! unknown market — but the call itself succeeds in exercising the
//! MCP path).
//!
//! Optional: `RUST_LOG=keeperhub_rs=debug cargo run --example morpho_position`
//! to see per-call tracing.

use keeperhub_rs::morpho::{compute_health_factor, wad_to_fraction, Morpho, MorphoAction};
use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use keeperhub_rs::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let api_key = std::env::var("KEEPERHUB_API_KEY").map_err(|_| {
        keeperhub_rs::Error::Config(
            "KEEPERHUB_API_KEY environment variable not set. \
             See plans/setup-verified.md for how to get one."
                .to_string(),
        )
    })?;

    let network = std::env::var("MORPHO_NETWORK").unwrap_or_else(|_| "1".into());
    let user = std::env::var("MORPHO_USER").map_err(|_| {
        keeperhub_rs::Error::Config(
            "MORPHO_USER must be set to the address to inspect.".to_string(),
        )
    })?;
    let market_id = std::env::var("MORPHO_MARKET_ID").unwrap_or_else(|_| {
        tracing::warn!(
            "MORPHO_MARKET_ID not set; using zero id. The call will likely \
             return a 'market not found' error from the plugin."
        );
        "0x0000000000000000000000000000000000000000000000000000000000000000".into()
    });

    let client = McpClient::new(DEFAULT_MCP_URL, &api_key);

    // Step 1: get market params (loanToken, collateralToken, oracle, irm, lltv).
    tracing::info!(%network, %market_id, "calling {}", MorphoAction::GetMarketParams);
    let params = Morpho::get_market_params(&client, &network, &market_id).await?;
    let pretty = serde_json::to_string_pretty(&params)
        .map_err(|e| keeperhub_rs::Error::Internal(format!("pretty-print: {e}")))?;
    println!("market params:\n{pretty}");

    // Step 2: get the user's position.
    tracing::info!(%user, "calling {}", MorphoAction::GetPosition);
    let position = Morpho::get_position(&client, &network, &market_id, &user).await?;
    let pretty = serde_json::to_string_pretty(&position)
        .map_err(|e| keeperhub_rs::Error::Internal(format!("pretty-print: {e}")))?;
    println!("position:\n{pretty}");

    // Step 3: compute health factor from the lltv + position. This is
    // pure-Rust math; the plugin doesn't compute HF for us. We
    // expect the caller to also fetch USD prices off-band (e.g. via
    // a Chainlink plugin call) to feed collateral_usd / debt_usd —
    // we illustrate the math with a sentinel example below.
    let lltv_wad = params
        .get("lltv")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let liq_threshold = wad_to_fraction(lltv_wad).unwrap_or(0.0);
    println!(
        "{}",
        json!({
            "lltv_wad": lltv_wad,
            "liq_threshold_fraction": liq_threshold,
            "note": "supply HF requires USD valuations of supplyShares \
                     and borrowShares, which the plugin does not return. \
                     Wire a price oracle (e.g. Chainlink) to feed \
                     compute_health_factor()."
        })
    );

    // Pure-math demo: same function, fake inputs.
    let demo_hf = compute_health_factor(1000.0, 500.0, liq_threshold.max(0.0));
    println!(
        "{}",
        json!({
            "demo_health_factor": demo_hf,
            "demo_assumption": "collateral=$1000, debt=$500, threshold=lltv",
        })
    );

    Ok(())
}
