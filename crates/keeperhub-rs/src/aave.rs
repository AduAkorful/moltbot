//! Typed helpers for the Aave V3 protocol plugin.
//!
//! Aave V3 is a KeeperHub **plugin** (not an integration type — see
//! `keeperhub-docs-summary.md` §5.1 architectural note from 2026-07-03).
//! Write actions need the org's existing `web3` wallet integration,
//! used implicitly when exactly one is configured. No separate per-org
//! "connect Aave" step is required.
//!
//! The helpers here wrap [`crate::mcp::McpClient::execute_protocol_action`]
//! with strongly-typed Rust signatures, so callers don't have to build
//! the `params` object by hand.
//!
//! # Action slugs
//!
//! The Aave V3 action slugs (e.g. `aave-v3/supply`, `aave-v3/withdraw`)
//! are defined in [`AaveV3Action`]. Pass one of those constants to
//! [`crate::mcp::McpClient::execute_protocol_action`] if you need the
//! raw escape hatch; otherwise prefer the typed helpers below.
//!
//! # Network format
//!
//! `network` is a **string** holding the chain ID — same shape used
//! by every other KeeperHub DeFi call. The Aave V3 plugin is deployed
//! on Ethereum (`"1"`), Base (`"8453"`), Arbitrum (`"42161"`), and
//! Optimism (`"10"`). Aave is **not** on Sepolia — there are no
//! testnet contracts.
//!
//! # Examples
//!
//! ```no_run
//! use keeperhub_rs::aave::AaveV3;
//! use keeperhub_rs::mcp::McpClient;
//!
//! # async fn demo() -> keeperhub_rs::Result<()> {
//! let client = McpClient::new("https://app.keeperhub.com/mcp", "kh_...");
//!
//! // Read-only: fetch account data (no onchain tx, no wallet needed).
//! let data = AaveV3::get_user_account_data(
//!     &client, "1", "0x54F9Fe5A1f63064fc083928df60A95db2dc2CE39"
//! ).await?;
//! println!("health factor: {}", data);
//!
//! // Write: supply 100 USDC (6 decimals = 100_000_000) on Ethereum mainnet.
//! let tx = AaveV3::supply(
//!     &client,
//!     "1",
//!     "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC on Ethereum
//!     "100000000",
//!     "0x54F9Fe5A1f63064fc083928df60A95db2dc2CE39", // on behalf of self
//!     0,                                            // no referral
//! ).await?;
//! println!("supply tx: {tx}");
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::mcp::McpClient;
use serde_json::{json, Value};

/// Action slugs for the Aave V3 plugin.
///
/// Pass one of these to
/// [`crate::mcp::McpClient::execute_protocol_action`] if you need the
/// raw escape hatch. Otherwise prefer the typed helpers on
/// [`AaveV3`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AaveV3Action {
    /// Supply an asset to the lending pool to earn interest (write).
    Supply,
    /// Withdraw a supplied asset (write).
    Withdraw,
    /// Borrow against supplied collateral (write).
    Borrow,
    /// Repay a borrowed asset (write).
    Repay,
    /// Enable or disable an asset as collateral (write).
    SetAsCollateral,
    /// Get overall account health: collateral, debt, HF (read).
    GetUserAccountData,
    /// Get per-asset position data and rates (read).
    GetUserReserveData,
}

impl AaveV3Action {
    /// The MCP action type slug, e.g. `"aave-v3/supply"`.
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::Supply => "aave-v3/supply",
            Self::Withdraw => "aave-v3/withdraw",
            Self::Borrow => "aave-v3/borrow",
            Self::Repay => "aave-v3/repay",
            Self::SetAsCollateral => "aave-v3/set-asset-as-collateral",
            Self::GetUserAccountData => "aave-v3/get-user-account-data",
            Self::GetUserReserveData => "aave-v3/get-user-reserve-data",
        }
    }
}

impl std::fmt::Display for AaveV3Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_slug())
    }
}

/// Typed helpers for the Aave V3 plugin.
///
/// All write helpers take a `network` string (chain ID as a string —
/// e.g. `"1"`, `"8453"`). The wallet is implicit when exactly one
/// `web3` integration is configured in the org, which is the common case
/// during the hackathon.
pub struct AaveV3;

impl AaveV3 {
    /// Supply `amount` of `asset` to the Aave V3 lending pool, on
    /// behalf of `on_behalf_of`.
    ///
    /// Returns the action's JSON output. The shape is documented in
    /// the KeeperHub Aave V3 plugin page; typically includes
    /// `success`, `transactionHash`, `transactionLink`, and `error`.
    ///
    /// # Arguments
    /// - `network` — chain ID as a string (e.g. `"1"` for Ethereum mainnet)
    /// - `asset` — token contract address
    /// - `amount` — amount in the asset's smallest unit (wei for 18-decimal
    ///   tokens; for USDC, 6 decimals — `"100000000"` = 100 USDC)
    /// - `on_behalf_of` — recipient of the aTokens. Use the user's own
    ///   address for self-supply
    /// - `referral_code` — referral code, `0` for none
    pub async fn supply(
        client: &McpClient,
        network: &str,
        asset: &str,
        amount: &str,
        on_behalf_of: &str,
        referral_code: u16,
    ) -> Result<Value> {
        client
            .execute_protocol_action(
                AaveV3Action::Supply.as_slug(),
                json!({
                    "network": network,
                    "asset": asset,
                    "amount": amount,
                    "onBehalfOf": on_behalf_of,
                    "referralCode": referral_code,
                }),
            )
            .await
    }

    /// Withdraw `amount` of `asset` from the Aave V3 lending pool, sent
    /// to `to`.
    ///
    /// # Arguments
    /// - `network` — chain ID as a string
    /// - `asset` — token contract address
    /// - `amount` — amount in the asset's smallest unit. Use
    ///   `"type(uint256).max"` to withdraw the full supplied balance
    ///   (the plugin passes the string through verbatim)
    /// - `to` — recipient address for the withdrawn tokens
    pub async fn withdraw(
        client: &McpClient,
        network: &str,
        asset: &str,
        amount: &str,
        to: &str,
    ) -> Result<Value> {
        client
            .execute_protocol_action(
                AaveV3Action::Withdraw.as_slug(),
                json!({
                    "network": network,
                    "asset": asset,
                    "amount": amount,
                    "to": to,
                }),
            )
            .await
    }

    /// Read overall Aave V3 account health for `user` on `network`.
    ///
    /// Returns the action's JSON output: `totalCollateralBase`,
    /// `totalDebtBase`, `availableBorrowsBase`, `currentLiquidationThreshold`,
    /// `ltv`, and `healthFactor` (the last is `1e18`-scaled — divide by
    /// `1e18` to get the human factor).
    ///
    /// This is a read-only call: no wallet, no gas, no onchain tx.
    pub async fn get_user_account_data(
        client: &McpClient,
        network: &str,
        user: &str,
    ) -> Result<Value> {
        client
            .execute_protocol_action(
                AaveV3Action::GetUserAccountData.as_slug(),
                json!({
                    "network": network,
                    "user": user,
                }),
            )
            .await
    }
}

/// Validate a chain-ID string for use in a protocol action `params.network`.
///
/// Returns the string unchanged if it parses as a non-negative integer,
/// or [`Error::Config`] otherwise. Empty strings and non-numeric input
/// are rejected — `network` must be a positive integer encoded as a
/// string per the API spec.
pub fn validate_network(network: &str) -> Result<&str> {
    if network.is_empty() {
        return Err(Error::Config("network must not be empty".to_string()));
    }
    if network.parse::<u64>().is_err() {
        return Err(Error::Config(format!(
            "network must be a numeric chain ID string, got {network:?}"
        )));
    }
    Ok(network)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aave_v3_action_slugs_match_documented_names() {
        assert_eq!(AaveV3Action::Supply.as_slug(), "aave-v3/supply");
        assert_eq!(AaveV3Action::Withdraw.as_slug(), "aave-v3/withdraw");
        assert_eq!(AaveV3Action::Borrow.as_slug(), "aave-v3/borrow");
        assert_eq!(AaveV3Action::Repay.as_slug(), "aave-v3/repay");
        assert_eq!(
            AaveV3Action::SetAsCollateral.as_slug(),
            "aave-v3/set-asset-as-collateral"
        );
        assert_eq!(
            AaveV3Action::GetUserAccountData.as_slug(),
            "aave-v3/get-user-account-data"
        );
        assert_eq!(
            AaveV3Action::GetUserReserveData.as_slug(),
            "aave-v3/get-user-reserve-data"
        );
    }

    #[test]
    fn aave_v3_action_display_uses_slug() {
        assert_eq!(format!("{}", AaveV3Action::Supply), "aave-v3/supply");
        assert_eq!(
            format!("{}", AaveV3Action::GetUserAccountData),
            "aave-v3/get-user-account-data"
        );
    }

    #[test]
    fn validate_network_accepts_numeric_strings() {
        assert_eq!(validate_network("1").unwrap(), "1");
        assert_eq!(validate_network("8453").unwrap(), "8453");
        assert_eq!(validate_network("11155111").unwrap(), "11155111");
        assert_eq!(validate_network("42161").unwrap(), "42161");
    }

    #[test]
    fn validate_network_rejects_empty() {
        let err = validate_network("").unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    #[test]
    fn validate_network_rejects_non_numeric() {
        let err = validate_network("ethereum").unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
        let err = validate_network("1.0").unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    #[test]
    fn validate_network_rejects_negative() {
        // The leading minus makes the parse fail for u64.
        let err = validate_network("-1").unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }
}
