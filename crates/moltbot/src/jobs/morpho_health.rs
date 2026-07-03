//! The Morpho health-factor job.
//!
//! On every tick where the job is enabled (i.e. `morpho_market_id`
//! is set in the config), this job reads the wallet's position in
//! a Morpho Blue market and decides whether to top it up with a
//! `morpho/supply-collateral` call.
//!
//! # Decision logic
//!
//! The decision is a pure function ([`decide`]) over an
//! `Option<f64>` health factor:
//!
//! | Observed HF              | Decision                                          |
//! |--------------------------|---------------------------------------------------|
//! | `None`                   | `InsufficientData` (could not compute HF)         |
//! | `+inf` (no debt)         | `NoPosition` (nothing to monitor)                 |
//! | `Some(h)`, `h < target`  | `Collateralize { current_hf: h, target_hf }`      |
//! | `Some(h)`, `h >= target` | `NoAction { current_hf: h }`                      |
//!
//! # Iteration #11 vs #17
//!
//! This iteration lands the **wiring** + the **pure decision
//! function**. Computing a real USD health factor from a Morpho
//! position requires oracle prices for both the collateral and
//! loan tokens; that lands in iteration #17 ("Wire Morpho job into
//! loop, real decision"). For #11, the job reads the position and
//! returns a read-only [`JobOutcome`] with a `note` summarizing the
//! raw `borrowShares` / `collateral` / `supplyShares` values. The
//! unit tests cover [`decide`] with mock health factors so the
//! logic is locked in before #17 plugs in the real HF source.

use keeperhub_rs::morpho::Morpho;
use keeperhub_rs::Result;
use serde_json::Value;

use crate::config::AgentConfig;
use crate::job::{BoxFuture, Job, JobContext, JobOutcome};
use crate::state::AgentState;

/// The Morpho health job's verdict for a single tick.
#[derive(Debug, Clone, PartialEq)]
pub enum MorphoDecision {
    /// HF is below the configured target — call
    /// `morpho/supply-collateral` to top up the position.
    Collateralize {
        current_hf: f64,
        target_hf: f64,
    },
    /// HF is at or above target — do nothing this tick.
    NoAction {
        current_hf: f64,
    },
    /// The position has no debt (no borrow) — nothing to monitor.
    NoPosition,
    /// The HF could not be computed (missing data, transient
    /// failure, etc.). The job should be retried next tick.
    InsufficientData,
}

impl std::fmt::Display for MorphoDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collateralize { current_hf, target_hf } => {
                write!(f, "collateralize(hf={:.3} < target={:.3})", current_hf, target_hf)
            }
            Self::NoAction { current_hf } => write!(f, "noop(hf={:.3})", current_hf),
            Self::NoPosition => write!(f, "noop(no position)"),
            Self::InsufficientData => write!(f, "noop(insufficient data)"),
        }
    }
}

/// Decide whether to collateralize given an observed health factor.
///
/// Pure function. `observed_hf` is `None` when the HF could not be
/// computed (e.g. position read failed, oracle offline, etc.).
/// `Some(f64::INFINITY)` is treated as "no debt" (no position to
/// monitor) and returns [`MorphoDecision::NoPosition`].
///
/// Boundary: `observed_hf == target_hf` is **not** collateralized
/// (strict `<`). The plan calls for `HF < 1.3 → collateralize`, so
/// `HF == 1.3` is safe.
pub fn decide(observed_hf: Option<f64>, target_hf: f64) -> MorphoDecision {
    match observed_hf {
        None => MorphoDecision::InsufficientData,
        Some(h) if h.is_infinite() && h > 0.0 => MorphoDecision::NoPosition,
        Some(h) if h < target_hf => MorphoDecision::Collateralize {
            current_hf: h,
            target_hf,
        },
        Some(h) => MorphoDecision::NoAction { current_hf: h },
    }
}

/// Read the `borrowShares` value from a `morpho/get-position`
/// response. Returns `true` when the position has no debt.
///
/// The Morpho plugin returns uint256 values as decimal strings,
/// so the comparison is string-based ("0" or "") to avoid
/// `u128`/`u256` parsing here.
fn position_has_no_debt(position: &Value) -> bool {
    let Some(bs) = position.get("borrowShares").and_then(|v| v.as_str()) else {
        // No field or non-string → treat as unknown / no debt
        return true;
    };
    bs.is_empty() || bs == "0"
}

/// Format a position response for the audit log.
fn format_position_note(position: &Value) -> String {
    let supply = position
        .get("supplyShares")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let borrow = position
        .get("borrowShares")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let collateral = position
        .get("collateral")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    format!(
        "morpho_position(supplyShares={supply}, borrowShares={borrow}, collateral={collateral})"
    )
}

/// The Morpho health-factor job.
#[derive(Debug, Default, Clone, Copy)]
pub struct MorphoHealthJob;

impl MorphoHealthJob {
    /// Create a new job. The job is a zero-sized type; construction
    /// is a no-op.
    pub fn new() -> Self {
        Self
    }
}

impl Job for MorphoHealthJob {
    fn name(&self) -> &'static str {
        "morpho_health"
    }

    fn should_run(&self, _state: &AgentState, config: &AgentConfig) -> bool {
        config.morpho_market_id.is_some()
    }

    fn tick<'a>(&'a self, ctx: &'a JobContext<'a>) -> BoxFuture<'a, Result<JobOutcome>> {
        Box::pin(async move {
            // The `should_run` gate guarantees `morpho_market_id`
            // is `Some`. We use `unwrap_or` defensively rather
            // than `unwrap` so an unexpected config change
            // produces a clean outcome rather than a panic.
            let Some(market_id) = ctx.config.morpho_market_id.as_deref() else {
                return Ok(JobOutcome::observed("morpho_market_id not set; job disabled"));
            };

            let position: Value = Morpho::get_position(
                ctx.client,
                &ctx.config.network,
                market_id,
                &ctx.config.wallet_address,
            )
            .await?;

            if position_has_no_debt(&position) {
                Ok(JobOutcome {
                    note: Some(format!("morpho_health: no position (borrowShares=0) on {market_id}")),
                    ..JobOutcome::default()
                })
            } else {
                // For #11, we don't yet compute a real USD health
                // factor (that lands in #17 with an oracle
                // integration). The job is read-only: log the
                // position state and return a NoPosition-style
                // "observed" outcome. The pure `decide` function
                // is unit-tested against mock HFs; the wiring
                // here proves the data flow end-to-end.
                Ok(JobOutcome::observed(format_position_note(&position)))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- decide() --------------------------------------------------------

    #[test]
    fn decide_collateralizes_below_target() {
        // Plan: HF < 1.3 → collateralize
        let d = decide(Some(1.2), 1.3);
        assert_eq!(
            d,
            MorphoDecision::Collateralize {
                current_hf: 1.2,
                target_hf: 1.3,
            }
        );
    }

    #[test]
    fn decide_collateralizes_way_below_target() {
        let d = decide(Some(0.5), 1.3);
        assert_eq!(
            d,
            MorphoDecision::Collateralize {
                current_hf: 0.5,
                target_hf: 1.3,
            }
        );
    }

    #[test]
    fn decide_does_nothing_at_target() {
        // Boundary: HF == target → NoAction (strict < for Collateralize)
        let d = decide(Some(1.3), 1.3);
        assert_eq!(d, MorphoDecision::NoAction { current_hf: 1.3 });
    }

    #[test]
    fn decide_does_nothing_above_target() {
        let d = decide(Some(1.5), 1.3);
        assert_eq!(d, MorphoDecision::NoAction { current_hf: 1.5 });
    }

    #[test]
    fn decide_way_above_target_noop() {
        let d = decide(Some(5.0), 1.3);
        assert_eq!(d, MorphoDecision::NoAction { current_hf: 5.0 });
    }

    #[test]
    fn decide_handles_no_debt() {
        // f64::INFINITY → no debt → nothing to monitor
        assert_eq!(
            decide(Some(f64::INFINITY), 1.3),
            MorphoDecision::NoPosition
        );
    }

    #[test]
    fn decide_handles_negative_infinity() {
        // Defensive: shouldn't happen, but the function shouldn't
        // classify it as "no position" (which is reserved for
        // positive infinity).
        let d = decide(Some(f64::NEG_INFINITY), 1.3);
        // NEG_INFINITY is < 1.3 → collateralize (the position is
        // deep underwater)
        assert_eq!(
            d,
            MorphoDecision::Collateralize {
                current_hf: f64::NEG_INFINITY,
                target_hf: 1.3,
            }
        );
    }

    #[test]
    fn decide_handles_zero_hf() {
        // HF = 0 → fully undercollateralized → collateralize
        let d = decide(Some(0.0), 1.3);
        assert_eq!(
            d,
            MorphoDecision::Collateralize {
                current_hf: 0.0,
                target_hf: 1.3,
            }
        );
    }

    #[test]
    fn decide_handles_missing_data() {
        assert_eq!(
            decide(None, 1.3),
            MorphoDecision::InsufficientData
        );
    }

    #[test]
    fn decide_respects_custom_target() {
        assert_eq!(
            decide(Some(1.4), 1.5),
            MorphoDecision::Collateralize {
                current_hf: 1.4,
                target_hf: 1.5,
            }
        );
        assert_eq!(
            decide(Some(1.6), 1.5),
            MorphoDecision::NoAction { current_hf: 1.6 }
        );
    }

    // --- position_has_no_debt() ------------------------------------------

    #[test]
    fn position_has_no_debt_for_zero_borrow_shares() {
        let v = serde_json::json!({
            "supplyShares": "0",
            "borrowShares": "0",
            "collateral": "0"
        });
        assert!(position_has_no_debt(&v));
    }

    #[test]
    fn position_has_no_debt_for_missing_borrow_shares() {
        let v = serde_json::json!({ "supplyShares": "0", "collateral": "0" });
        assert!(position_has_no_debt(&v));
    }

    #[test]
    fn position_has_no_debt_for_empty_borrow_shares() {
        let v = serde_json::json!({ "borrowShares": "" });
        assert!(position_has_no_debt(&v));
    }

    #[test]
    fn position_has_no_debt_for_non_string_borrow_shares() {
        // Defensive: if the plugin returns a non-string, treat as
        // "no data" → no debt (safer to skip than to overact).
        let v = serde_json::json!({ "borrowShares": 42 });
        assert!(position_has_no_debt(&v));
    }

    #[test]
    fn position_has_debt_for_nonzero_borrow_shares() {
        let v = serde_json::json!({
            "supplyShares": "1000000",
            "borrowShares": "500000",
            "collateral": "2000000"
        });
        assert!(!position_has_no_debt(&v));
    }

    // --- format_position_note() ------------------------------------------

    #[test]
    fn format_position_note_includes_all_fields() {
        let v = serde_json::json!({
            "supplyShares": "100",
            "borrowShares": "50",
            "collateral": "200"
        });
        let note = format_position_note(&v);
        assert!(note.contains("supplyShares=100"));
        assert!(note.contains("borrowShares=50"));
        assert!(note.contains("collateral=200"));
    }

    #[test]
    fn format_position_note_handles_missing_fields() {
        let v = serde_json::json!({});
        let note = format_position_note(&v);
        assert!(note.contains("supplyShares=?"));
        assert!(note.contains("borrowShares=?"));
        assert!(note.contains("collateral=?"));
    }

    // --- MorphoHealthJob::should_run() -----------------------------------

    #[test]
    fn should_run_is_false_when_market_id_missing() {
        let job = MorphoHealthJob::new();
        let cfg = AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            ..AgentConfig::default()
        };
        assert!(!job.should_run(&AgentState::new(), &cfg));
    }

    #[test]
    fn should_run_is_true_when_market_id_set() {
        let job = MorphoHealthJob::new();
        let cfg = AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            morpho_market_id: Some(
                "0x54efc345a0180ad8a99ae62b1c626e0d2e46a4d3936d36e8b54df7fb3d0c1b8f"
                    .to_string(),
            ),
            ..AgentConfig::default()
        };
        assert!(job.should_run(&AgentState::new(), &cfg));
    }

    // --- MorphoDecision Display ------------------------------------------

    #[test]
    fn morpho_decision_display() {
        assert_eq!(
            MorphoDecision::Collateralize {
                current_hf: 1.21,
                target_hf: 1.3
            }
            .to_string(),
            "collateralize(hf=1.210 < target=1.300)"
        );
        assert_eq!(
            MorphoDecision::NoAction { current_hf: 1.5 }.to_string(),
            "noop(hf=1.500)"
        );
        assert_eq!(MorphoDecision::NoPosition.to_string(), "noop(no position)");
        assert_eq!(
            MorphoDecision::InsufficientData.to_string(),
            "noop(insufficient data)"
        );
    }
}
