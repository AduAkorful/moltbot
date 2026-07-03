//! Yield strategy: park idle USDC into Aave V3, pull back when low.
//!
//! The strategy is a tiny state machine: read the wallet's current
//! USDC balance, compare it against the configured
//! [`AgentConfig::park_threshold_usd`] and
//! [`AgentConfig::withdraw_threshold_usd`], and either supply the
//! full balance, withdraw everything from Aave, or do nothing.
//!
//! # Decision table
//!
//! | Balance vs thresholds        | Decision                |
//! |------------------------------|-------------------------|
//! | `balance > park`             | `Supply { amount }`     |
//! | `balance < withdraw`         | `Withdraw`              |
//! | `withdraw <= balance <= park`| `NoAction`              |
//!
//! Boundaries are **strict**: balance exactly at `park` or `withdraw`
//! is `NoAction`. This avoids flip-flopping on the boundary.
//!
//! # What this module does NOT do
//!
//! - It does **not** poll the wallet for the live USDC balance. The
//!   caller is responsible for updating
//!   [`crate::state::AgentState::usdc_balance_usd`] before invoking
//!   [`decide`].
//! - It does **not** compute a specific withdraw amount. `Withdraw`
//!   always passes `"type(uint256).max"` to the Aave plugin, which
//!   pulls the entire aToken balance back to the wallet.
//! - It does **not** decide whether the agent is in safe mode. The
//!   loop in [`crate::tick::AgentLoop::tick`] short-circuits yield
//!   entirely when `state.safe_mode` is `true`; this module is
//!   only consulted when the agent is operational.
//!
//! # Layering
//!
//! The pure decision function [`decide`] has no I/O and is unit-tested
//! with mocked balances. The I/O lives in [`execute`], which calls the
//! KeeperHub MCP via [`keeperhub_rs::aave::AaveV3`].

use crate::config::AgentConfig;
use keeperhub_rs::aave::AaveV3;
use keeperhub_rs::mcp::McpClient;
use keeperhub_rs::Result;
use serde_json::Value;
use std::fmt;

/// The yield strategy's verdict for a single tick.
///
/// Cheap to construct, clone, and compare. [`Display`] is a short
/// human-readable form used in tracing logs.
#[derive(Debug, Clone, PartialEq)]
pub enum ParkDecision {
    /// Supply `amount_usd` of USDC to Aave V3. The agent's wallet is
    /// the supplier; the resulting aTokens accrue to the same
    /// wallet.
    Supply {
        /// Amount in human-readable USD (e.g. `50.0` for $50). The
        /// conversion to USDC's 6-decimal smallest unit happens in
        /// [`execute`].
        amount_usd: f64,
    },
    /// Withdraw the full aToken balance from Aave V3 back to the
    /// wallet. Implemented as `"type(uint256).max"` in the Aave
    /// plugin's `amount` param.
    Withdraw,
    /// Do nothing this tick.
    NoAction,
}

impl fmt::Display for ParkDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Supply { amount_usd } => write!(f, "supply(${:.2})", amount_usd),
            Self::Withdraw => write!(f, "withdraw(all)"),
            Self::NoAction => write!(f, "noop"),
        }
    }
}

/// Decide what to do with the wallet's current USDC balance.
///
/// Pure function. `park_threshold_usd` and `withdraw_threshold_usd`
/// are both assumed to be in the same unit as `balance_usd` (human
/// USD). Callers should ensure
/// `park_threshold_usd >= withdraw_threshold_usd` (validated by
/// [`AgentConfig::validate`]).
pub fn decide(
    balance_usd: f64,
    park_threshold_usd: f64,
    withdraw_threshold_usd: f64,
) -> ParkDecision {
    if balance_usd > park_threshold_usd {
        ParkDecision::Supply {
            amount_usd: balance_usd,
        }
    } else if balance_usd < withdraw_threshold_usd {
        ParkDecision::Withdraw
    } else {
        ParkDecision::NoAction
    }
}

/// Convert a human-readable USD amount to USDC's 6-decimal smallest
/// unit, as a decimal string suitable for the Aave V3 plugin.
///
/// Negative inputs clamp to `"0"`. Fractional cents floor to avoid
/// sending dust that the contract would round-reject. `NaN` is not
/// representable in JSON and isn't expected here; the function would
/// produce `"NaN"`, which the Aave plugin would reject — this is
/// intentional (fail loud at the network boundary).
pub fn usd_to_usdc_smallest_unit(usd: f64) -> String {
    if usd <= 0.0 {
        return "0".to_string();
    }
    let units = (usd * 1_000_000.0).floor();
    // `f64` is fine here: even at $1e9 USDC the integer count fits
    // in 2^53 with plenty of headroom. The Aave plugin will reject
    // anything that doesn't parse as a uint256, so we keep the
    // string clean.
    format!("{}", units as u64)
}

/// Extract a transaction hash from a parsed Aave V3 plugin response.
///
/// The Aave V3 plugin returns a JSON object with shape:
/// ```json
/// { "success": true, "transactionHash": "0x...", "transactionLink": "...", "error": null }
/// ```
/// We also accept `"txHash"` and `"hash"` as fallbacks for plugin
/// versions that may use different key names. Returns `None` for
/// missing fields, non-string fields, or non-object payloads.
pub fn extract_tx_hash(result: &Value) -> Option<String> {
    let obj = result.as_object()?;
    for key in ["transactionHash", "txHash", "hash"] {
        if let Some(Value::String(s)) = obj.get(key) {
            return Some(s.clone());
        }
    }
    None
}

/// Execute a [`ParkDecision`] against the live KeeperHub MCP server.
///
/// Returns `Ok(Some(tx_hash))` for `Supply` and `Withdraw` decisions
/// when the plugin response includes a transaction hash,
/// `Ok(None)` for `NoAction`, and `Ok(None)` for `Supply`/`Withdraw`
/// responses that don't include a hash (logged as a warning by the
/// caller). Network and parsing errors bubble up as
/// [`keeperhub_rs::Error`].
pub async fn execute(
    client: &McpClient,
    config: &AgentConfig,
    decision: &ParkDecision,
) -> Result<Option<String>> {
    match decision {
        ParkDecision::Supply { amount_usd } => {
            let amount = usd_to_usdc_smallest_unit(*amount_usd);
            let result = AaveV3::supply(
                client,
                &config.network,
                &config.usdc_address,
                &amount,
                &config.wallet_address,
                0,
            )
            .await?;
            Ok(extract_tx_hash(&result))
        }
        ParkDecision::Withdraw => {
            let result = AaveV3::withdraw(
                client,
                &config.network,
                &config.usdc_address,
                "type(uint256).max",
                &config.wallet_address,
            )
            .await?;
            Ok(extract_tx_hash(&result))
        }
        ParkDecision::NoAction => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reasonable defaults that match AgentConfig's defaults.
    const PARK: f64 = 50.0;
    const WITHDRAW: f64 = 20.0;

    // --- decide() --------------------------------------------------------

    #[test]
    fn decide_way_above_park_supplies() {
        // balance = 4x park → Supply the full balance
        let d = decide(200.0, PARK, WITHDRAW);
        assert_eq!(d, ParkDecision::Supply { amount_usd: 200.0 });
    }

    #[test]
    fn decide_just_above_park_supplies() {
        // 1 cent above park → Supply
        let d = decide(50.01, PARK, WITHDRAW);
        assert_eq!(d, ParkDecision::Supply { amount_usd: 50.01 });
    }

    #[test]
    fn decide_between_thresholds_is_noop() {
        // Midpoint between withdraw and park
        assert_eq!(decide(35.0, PARK, WITHDRAW), ParkDecision::NoAction);
    }

    #[test]
    fn decide_just_below_withdraw_pulls() {
        // 1 cent below withdraw → Withdraw
        assert_eq!(decide(19.99, PARK, WITHDRAW), ParkDecision::Withdraw);
    }

    #[test]
    fn decide_way_below_withdraw_pulls() {
        // Well above safe-mode floor ($5) but well below withdraw
        assert_eq!(decide(6.0, PARK, WITHDRAW), ParkDecision::Withdraw);
    }

    #[test]
    fn decide_at_park_threshold_is_noop() {
        // Boundary: balance == park → NoAction (strict > for Supply)
        assert_eq!(decide(50.0, PARK, WITHDRAW), ParkDecision::NoAction);
    }

    #[test]
    fn decide_at_withdraw_threshold_is_noop() {
        // Boundary: balance == withdraw → NoAction (strict < for Withdraw)
        assert_eq!(decide(20.0, PARK, WITHDRAW), ParkDecision::NoAction);
    }

    #[test]
    fn decide_at_zero_balance_pulls() {
        // No USDC left → Withdraw to refill from Aave
        assert_eq!(decide(0.0, PARK, WITHDRAW), ParkDecision::Withdraw);
    }

    #[test]
    fn decide_with_negative_balance_pulls() {
        // Defensive: shouldn't happen, but be safe
        assert_eq!(decide(-5.0, PARK, WITHDRAW), ParkDecision::Withdraw);
    }

    #[test]
    fn decide_respects_custom_thresholds() {
        // Different config: tighter band
        assert_eq!(decide(15.0, 10.0, 5.0), ParkDecision::Supply { amount_usd: 15.0 });
        assert_eq!(decide(7.0, 10.0, 5.0), ParkDecision::NoAction);
        assert_eq!(decide(4.0, 10.0, 5.0), ParkDecision::Withdraw);
    }

    // --- usd_to_usdc_smallest_unit() -------------------------------------

    #[test]
    fn usdc_smallest_unit_for_common_amounts() {
        assert_eq!(usd_to_usdc_smallest_unit(1.0), "1000000");
        assert_eq!(usd_to_usdc_smallest_unit(100.0), "100000000");
        assert_eq!(usd_to_usdc_smallest_unit(0.5), "500000");
    }

    #[test]
    fn usdc_smallest_unit_floors_fractional_cents() {
        // 0.0000001 USDC = 0.1 of the smallest unit → floor to 0
        assert_eq!(usd_to_usdc_smallest_unit(0.0000001), "0");
    }

    #[test]
    fn usdc_smallest_unit_clamps_nonpositive_to_zero() {
        assert_eq!(usd_to_usdc_smallest_unit(0.0), "0");
        assert_eq!(usd_to_usdc_smallest_unit(-1.0), "0");
    }

    // --- extract_tx_hash() ----------------------------------------------

    #[test]
    fn extract_tx_hash_finds_transaction_hash() {
        let v = serde_json::json!({
            "success": true,
            "transactionHash": "0xabc123",
            "transactionLink": "https://etherscan.io/tx/0xabc123"
        });
        assert_eq!(extract_tx_hash(&v), Some("0xabc123".to_string()));
    }

    #[test]
    fn extract_tx_hash_uses_alternate_keys() {
        // Older plugin versions may use a different key name
        let v = serde_json::json!({ "txHash": "0xdef" });
        assert_eq!(extract_tx_hash(&v), Some("0xdef".to_string()));
        let v = serde_json::json!({ "hash": "0xghi" });
        assert_eq!(extract_tx_hash(&v), Some("0xghi".to_string()));
    }

    #[test]
    fn extract_tx_hash_returns_none_for_missing_field() {
        let v = serde_json::json!({ "success": true });
        assert_eq!(extract_tx_hash(&v), None);
    }

    #[test]
    fn extract_tx_hash_returns_none_for_non_string_field() {
        let v = serde_json::json!({ "transactionHash": 42 });
        assert_eq!(extract_tx_hash(&v), None);
    }

    #[test]
    fn extract_tx_hash_handles_null_payload() {
        assert_eq!(extract_tx_hash(&Value::Null), None);
    }

    #[test]
    fn extract_tx_hash_handles_array_payload() {
        // Non-object: not a valid plugin response, but be defensive
        let v = serde_json::json!([]);
        assert_eq!(extract_tx_hash(&v), None);
    }

    // --- Display ---------------------------------------------------------

    #[test]
    fn park_decision_display() {
        assert_eq!(
            ParkDecision::Supply { amount_usd: 50.0 }.to_string(),
            "supply($50.00)"
        );
        assert_eq!(
            ParkDecision::Supply { amount_usd: 200.123 }.to_string(),
            "supply($200.12)"
        );
        assert_eq!(ParkDecision::Withdraw.to_string(), "withdraw(all)");
        assert_eq!(ParkDecision::NoAction.to_string(), "noop");
    }
}
