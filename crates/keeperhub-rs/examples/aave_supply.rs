//! Example: supply USDC to Aave V3 via `execute_protocol_action`.
//!
//! Demonstrates the typed Aave V3 helpers in [`keeperhub_rs::aave`].
//! This actually moves tokens onchain (on whichever network you point
//! it at), so use a testnet address and a small amount first.
//!
//! # Usage
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_your_key_here
//! export AAVE_NETWORK=1                  # Ethereum mainnet
//! export AAVE_ASSET=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48  # USDC
//! export AAVE_AMOUNT=1000000             # 1 USDC (6 decimals)
//! export AAVE_ON_BEHALF_OF=0xYourAddress
//! cargo run --example aave_supply
//! ```
//!
//! To *only* read the user's Aave account data (no onchain tx, no
//! wallet, no money), set `AAVE_DRY_RUN=1`. The example will call
//! `aave-v3/get-user-account-data` instead of supply and print the
//! result.
//!
//! Optional: `RUST_LOG=keeperhub_rs=debug cargo run --example aave_supply`
//! to see per-call tracing.

use keeperhub_rs::aave::AaveV3;
use keeperhub_rs::aave::AaveV3Action;
use keeperhub_rs::mcp::{McpClient, DEFAULT_MCP_URL};
use keeperhub_rs::Result;

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

    let network = std::env::var("AAVE_NETWORK").unwrap_or_else(|_| "1".into());
    let asset = std::env::var("AAVE_ASSET").unwrap_or_else(|_| {
        // USDC on Ethereum mainnet.
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into()
    });
    let amount = std::env::var("AAVE_AMOUNT").unwrap_or_else(|_| "1000000".into()); // 1 USDC
    let on_behalf_of = std::env::var("AAVE_ON_BEHALF_OF").map_err(|_| {
        keeperhub_rs::Error::Config(
            "AAVE_ON_BEHALF_OF must be set to the address that will receive the aTokens \
             (typically your own wallet)."
                .to_string(),
        )
    })?;
    let dry_run = std::env::var("AAVE_DRY_RUN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let client = McpClient::new(DEFAULT_MCP_URL, &api_key);

    if dry_run {
        tracing::info!(
            network = %network,
            user = %on_behalf_of,
            "AAVE_DRY_RUN=1; calling {} instead of supply",
            AaveV3Action::GetUserAccountData,
        );
        let data = AaveV3::get_user_account_data(&client, &network, &on_behalf_of).await?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data)
                .map_err(|e| keeperhub_rs::Error::Internal(format!("pretty-print failed: {e}")))?
        );
        return Ok(());
    }

    tracing::warn!(
        network = %network,
        asset = %asset,
        amount = %amount,
        on_behalf_of = %on_behalf_of,
        "LIVE AAVE SUPPLY: this will broadcast a transaction"
    );
    let result = AaveV3::supply(&client, &network, &asset, &amount, &on_behalf_of, 0).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result)
            .map_err(|e| keeperhub_rs::Error::Internal(format!("pretty-print failed: {e}")))?
    );
    Ok(())
}
