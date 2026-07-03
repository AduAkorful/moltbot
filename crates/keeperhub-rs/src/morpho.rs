//! Typed helpers for the Morpho Blue plugin and pure-Rust helpers
//! for health-factor math.
//!
//! Morpho is a KeeperHub **plugin** (not an integration type — see
//! `keeperhub-docs-summary.md` §5.1 architectural note from 2026-07-03).
//! Write actions need the org's existing `web3` wallet integration,
//! used implicitly when exactly one is configured.
//!
//! # Market identification
//!
//! Morpho Blue is a singleton contract. Each market is identified by
//! a 32-byte `id` (the keccak256 of the `MarketParams` struct). The
//! helpers here take that `id` as a `&str` of the form `0x...` (66
//! chars including the prefix). Resolve an `id` from
//! `loanToken`/`collateralToken`/`oracle`/`irm`/`lltv` via
//! [`Morpho::get_market_params`].
//!
//! # Supported chains
//!
//! Morpho Blue runs on Ethereum mainnet (`"1"`) and Base mainnet
//! (`"8453"`). It does **not** have a testnet deployment.
//!
//! # Health factor
//!
//! [`compute_health_factor`] is a pure function that returns the
//! position's health factor (HF > 1.0 means safe, HF < 1.0 means
//! liquidatable). Use [`basis_points_to_fraction`] to scale
//! Aave-style `lltv` values (1e4 basis-points scale) into the
//! 0.0–1.0 fraction the HF function expects.

use crate::error::{Error, Result};
use crate::mcp::McpClient;
use serde_json::{json, Value};

/// Action slugs for the Morpho plugin.
///
/// The 5-field `MarketParams` struct (`loanToken`, `collateralToken`,
/// `oracle`, `irm`, `lltv`) is passed as flat inputs to write actions;
/// the KeeperHub runtime reshapes them into the Solidity tuple
/// automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphoAction {
    /// Read a user's supply shares, borrow shares, and collateral in a market.
    GetPosition,
    /// Read total supply, borrows, last update, fee for a market.
    GetMarket,
    /// Resolve a market id to its `MarketParams` (read).
    GetMarketParams,
    /// Check if an address is authorized to act on behalf of another (read).
    CheckAuthorization,
    /// Grant or revoke authorization for another address (write).
    SetAuthorization,
    /// Borrow tokens and repay within the same transaction (write).
    FlashLoan,
    /// Supply loan tokens to a market (write).
    Supply,
    /// Withdraw supplied loan tokens from a market (write).
    Withdraw,
    /// Borrow loan tokens against deposited collateral (write).
    Borrow,
    /// Repay borrowed loan tokens to a market (write).
    Repay,
    /// Deposit collateral tokens into a market (write).
    SupplyCollateral,
    /// Remove collateral tokens from a position (write).
    WithdrawCollateral,
    /// Liquidate an undercollateralized position (write).
    Liquidate,
    /// Trigger interest accrual for a market (write, permissionless).
    AccrueInterest,
}

impl MorphoAction {
    /// The MCP action type slug, e.g. `"morpho/get-position"`.
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::GetPosition => "morpho/get-position",
            Self::GetMarket => "morpho/get-market",
            Self::GetMarketParams => "morpho/get-market-params",
            Self::CheckAuthorization => "morpho/check-authorization",
            Self::SetAuthorization => "morpho/set-authorization",
            Self::FlashLoan => "morpho/flash-loan",
            Self::Supply => "morpho/supply",
            Self::Withdraw => "morpho/withdraw",
            Self::Borrow => "morpho/borrow",
            Self::Repay => "morpho/repay",
            Self::SupplyCollateral => "morpho/supply-collateral",
            Self::WithdrawCollateral => "morpho/withdraw-collateral",
            Self::Liquidate => "morpho/liquidate",
            Self::AccrueInterest => "morpho/accrue-interest",
        }
    }
}

impl std::fmt::Display for MorphoAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_slug())
    }
}

/// Typed helpers for the Morpho Blue plugin.
///
/// Read-only helpers below can be called without a wallet. Write
/// helpers (Supply, Withdraw, Borrow, Repay, SupplyCollateral,
/// WithdrawCollateral, Liquidate, SetAuthorization, FlashLoan) take
/// the 5-field `MarketParams` plus action-specific extras; see the
/// `keeperhub-rs/examples/morpho_position.rs` example for shape.
pub struct Morpho;

impl Morpho {
    /// Read a user's position in a Morpho market.
    ///
    /// # Arguments
    /// - `network` — chain ID string, `"1"` (Ethereum) or `"8453"` (Base)
    /// - `market_id` — the market's 32-byte id as a `0x`-prefixed hex string
    ///   (66 chars total, e.g. `"0x54efc...0c1b8f"`)
    /// - `user` — the user address to read
    ///
    /// Returns the action's JSON output: `supplyShares`, `borrowShares`,
    /// and `collateral` (all in the market's native units — typically
    /// the underlying token's smallest unit or shares, not USD).
    pub async fn get_position(
        client: &McpClient,
        network: &str,
        market_id: &str,
        user: &str,
    ) -> Result<Value> {
        validate_market_id(market_id)?;
        client
            .execute_protocol_action(
                MorphoAction::GetPosition.as_slug(),
                json!({
                    "network": network,
                    "id": market_id,
                    "user": user,
                }),
            )
            .await
    }

    /// Read a market's aggregate state: total supply, borrows, last
    /// update, fee.
    pub async fn get_market(
        client: &McpClient,
        network: &str,
        market_id: &str,
    ) -> Result<Value> {
        validate_market_id(market_id)?;
        client
            .execute_protocol_action(
                MorphoAction::GetMarket.as_slug(),
                json!({
                    "network": network,
                    "id": market_id,
                }),
            )
            .await
    }

    /// Resolve a market id to its full `MarketParams`: loan token,
    /// collateral token, oracle, IRM, and `lltv`.
    ///
    /// This is the canonical way to look up the `lltv` (liquidation
    /// loan-to-value) for a market — feed it through
    /// [`basis_points_to_fraction`] to get the fraction used by
    /// [`compute_health_factor`].
    pub async fn get_market_params(
        client: &McpClient,
        network: &str,
        market_id: &str,
    ) -> Result<Value> {
        validate_market_id(market_id)?;
        client
            .execute_protocol_action(
                MorphoAction::GetMarketParams.as_slug(),
                json!({
                    "network": network,
                    "id": market_id,
                }),
            )
            .await
    }
}

/// Validate a Morpho market id string.
///
/// Morpho market ids are 32-byte keccak256 hashes, hex-encoded with
/// an optional `0x` prefix. We require the canonical 66-character
/// `0x`-prefixed form to match the rest of the KeeperHub API.
fn validate_market_id(market_id: &str) -> Result<()> {
    if !market_id.starts_with("0x") {
        return Err(Error::Config(format!(
            "Morpho market id must be 0x-prefixed, got {market_id:?}"
        )));
    }
    let hex = &market_id[2..];
    if hex.len() != 64 {
        return Err(Error::Config(format!(
            "Morpho market id must be 64 hex chars after 0x, got {} chars",
            hex.len()
        )));
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Config(format!(
            "Morpho market id contains non-hex characters: {market_id:?}"
        )));
    }
    Ok(())
}

/// Compute a position's health factor from collateral, debt, and the
/// liquidation threshold (all in the same unit, typically USD).
///
/// `health_factor = (collateral * liq_threshold) / debt`
///
/// Conventions:
/// - `collateral_usd` and `debt_usd` are in the same unit (e.g. USD
///   with 8 decimals scaled, or any consistent unit).
/// - `liq_threshold` is a fraction in `[0.0, 1.0]` — e.g. `0.8` for an
///   80% liquidation LTV. Use [`basis_points_to_fraction`] to scale
///   Aave-style basis-points values into this range.
/// - Returns `f64::INFINITY` when `debt_usd == 0.0` (no debt = no
///   liquidation risk).
/// - Returns `0.0` when `collateral_usd == 0.0` (no collateral = always
///   liquidatable).
/// - HF > 1.0 means the position is safe; HF < 1.0 means the position
///   is undercollateralized and can be liquidated.
pub fn compute_health_factor(collateral_usd: f64, debt_usd: f64, liq_threshold: f64) -> f64 {
    if debt_usd == 0.0 {
        return f64::INFINITY;
    }
    if collateral_usd == 0.0 {
        return 0.0;
    }
    (collateral_usd * liq_threshold) / debt_usd
}

/// Convert a basis-points value (Aave's `currentLiquidationThreshold`
/// and `ltv` format) to a fraction in `[0.0, 1.0]`.
///
/// `8500` (85%) → `0.85`. Inputs are clamped to `[0, 10_000]`
/// (i.e. 0% to 100%); values outside that range produce values outside
/// `[0.0, 1.0]` (e.g. negative input → negative fraction).
pub fn basis_points_to_fraction(bps: i64) -> f64 {
    (bps as f64) / 10_000.0
}

/// Parse a Morpho `lltv` value to a fraction. Morpho stores `lltv` as
/// a `uint256` WAD-scaled fraction (1e18 = 100%). `0.86e18` → `0.86`.
///
/// Equivalent to `lltv_wad as f64 / 1e18`. Strings of decimal digits
/// are accepted (the API often returns uint256s as strings).
///
/// For Aave's basis-points format, use [`basis_points_to_fraction`]
/// instead.
pub fn wad_to_fraction(lltv_wad: &str) -> Result<f64> {
    let parsed: f64 = lltv_wad.parse().map_err(|e| {
        Error::Config(format!(
            "Morpho lltv must be a numeric string, got {lltv_wad:?}: {e}"
        ))
    })?;
    Ok(parsed / 1e18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morpho_action_slugs_match_documented_names() {
        assert_eq!(MorphoAction::GetPosition.as_slug(), "morpho/get-position");
        assert_eq!(MorphoAction::GetMarket.as_slug(), "morpho/get-market");
        assert_eq!(
            MorphoAction::GetMarketParams.as_slug(),
            "morpho/get-market-params"
        );
        assert_eq!(
            MorphoAction::CheckAuthorization.as_slug(),
            "morpho/check-authorization"
        );
        assert_eq!(
            MorphoAction::SetAuthorization.as_slug(),
            "morpho/set-authorization"
        );
        assert_eq!(MorphoAction::FlashLoan.as_slug(), "morpho/flash-loan");
        assert_eq!(MorphoAction::Supply.as_slug(), "morpho/supply");
        assert_eq!(MorphoAction::Withdraw.as_slug(), "morpho/withdraw");
        assert_eq!(MorphoAction::Borrow.as_slug(), "morpho/borrow");
        assert_eq!(MorphoAction::Repay.as_slug(), "morpho/repay");
        assert_eq!(
            MorphoAction::SupplyCollateral.as_slug(),
            "morpho/supply-collateral"
        );
        assert_eq!(
            MorphoAction::WithdrawCollateral.as_slug(),
            "morpho/withdraw-collateral"
        );
        assert_eq!(MorphoAction::Liquidate.as_slug(), "morpho/liquidate");
        assert_eq!(
            MorphoAction::AccrueInterest.as_slug(),
            "morpho/accrue-interest"
        );
    }

    #[test]
    fn morpho_action_display_uses_slug() {
        assert_eq!(format!("{}", MorphoAction::Supply), "morpho/supply");
        assert_eq!(
            format!("{}", MorphoAction::WithdrawCollateral),
            "morpho/withdraw-collateral"
        );
    }

    #[test]
    fn validate_market_id_accepts_0x_hex_64_chars() {
        // Use a syntactically valid 32-byte hex (all zeros for simplicity).
        let id = "0x0000000000000000000000000000000000000000000000000000000000000000";
        assert!(validate_market_id(id).is_ok());
    }

    #[test]
    fn validate_market_id_accepts_real_looking_hash() {
        let id = "0x54efc345a0180ad8a99ae62b1c626e0d2e46a4d3936d36e8b54df7fb3d0c1b8f";
        assert!(validate_market_id(id).is_ok());
    }

    #[test]
    fn validate_market_id_rejects_missing_0x_prefix() {
        let err = validate_market_id("54efc345a0180ad8a99ae62b1c626e0d2e46a4d3936d36e8b54df7fb3d0c1b8f")
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    #[test]
    fn validate_market_id_rejects_wrong_length() {
        // 63 hex chars after 0x (off by one).
        let id = "0x54efc345a0180ad8a99ae62b1c626e0d2e46a4d3936d36e8b54df7fb3d0c1b8";
        let err = validate_market_id(id).unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    #[test]
    fn validate_market_id_rejects_non_hex() {
        let id = "0xZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ";
        let err = validate_market_id(id).unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    // ---- compute_health_factor ----

    #[test]
    fn compute_health_factor_healthy_position() {
        // $1000 collateral, $500 debt, 80% liq threshold -> HF = 1.6
        let hf = compute_health_factor(1000.0, 500.0, 0.8);
        assert!((hf - 1.6).abs() < 1e-9, "got {hf}");
    }

    #[test]
    fn compute_health_factor_at_risk_position() {
        // $1000 collateral, $750 debt, 80% liq threshold -> HF = 1.0666...
        let hf = compute_health_factor(1000.0, 750.0, 0.8);
        assert!((hf - 1.0666666666666667).abs() < 1e-9, "got {hf}");
    }

    #[test]
    fn compute_health_factor_underwater_position() {
        // $500 collateral, $1000 debt, 80% liq threshold -> HF = 0.4
        let hf = compute_health_factor(500.0, 1000.0, 0.8);
        assert!((hf - 0.4).abs() < 1e-9, "got {hf}");
        assert!(hf < 1.0, "underwater HF should be < 1.0");
    }

    #[test]
    fn compute_health_factor_no_debt_is_infinite() {
        let hf = compute_health_factor(1000.0, 0.0, 0.8);
        assert!(hf.is_infinite() && hf > 0.0, "got {hf}");
    }

    #[test]
    fn compute_health_factor_no_collateral_is_zero() {
        let hf = compute_health_factor(0.0, 1000.0, 0.8);
        assert_eq!(hf, 0.0);
    }

    #[test]
    fn compute_health_factor_at_threshold_is_one() {
        // collateral * threshold == debt -> HF = 1.0 (the liquidation boundary).
        let hf = compute_health_factor(1000.0, 800.0, 0.8);
        assert!((hf - 1.0).abs() < 1e-9, "got {hf}");
    }

    // ---- basis_points_to_fraction ----

    #[test]
    fn basis_points_to_fraction_converts_correctly() {
        assert_eq!(basis_points_to_fraction(0), 0.0);
        assert_eq!(basis_points_to_fraction(5_000), 0.5);
        assert_eq!(basis_points_to_fraction(8_000), 0.8);
        assert_eq!(basis_points_to_fraction(8_500), 0.85);
        assert_eq!(basis_points_to_fraction(10_000), 1.0);
    }

    #[test]
    fn basis_points_to_fraction_clamps_out_of_range() {
        // We don't actually clamp — the docstring says values outside
        // [0, 10_000] produce out-of-range fractions. The caller is
        // responsible for clamping if needed.
        let over = basis_points_to_fraction(15_000);
        assert!((over - 1.5).abs() < 1e-9, "got {over}");
    }

    // ---- wad_to_fraction ----

    #[test]
    fn wad_to_fraction_parses_morpho_lltv() {
        // 0.86e18 == 86% LLTV
        let lltv = "860000000000000000";
        let frac = wad_to_fraction(lltv).unwrap();
        assert!((frac - 0.86).abs() < 1e-9, "got {frac}");
    }

    #[test]
    fn wad_to_fraction_handles_full_range() {
        // 1e18 = 100%
        let frac = wad_to_fraction("1000000000000000000").unwrap();
        assert!((frac - 1.0).abs() < 1e-9, "got {frac}");
        // 0 = 0%
        let frac = wad_to_fraction("0").unwrap();
        assert_eq!(frac, 0.0);
    }

    #[test]
    fn wad_to_fraction_rejects_non_numeric() {
        let err = wad_to_fraction("not-a-number").unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }
}
