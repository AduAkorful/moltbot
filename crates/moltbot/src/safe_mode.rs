//! Safe-mode: low-balance detection.
//!
//! When the wallet's USDC balance drops below
//! [`AgentConfig::safe_mode_threshold_usd`], the agent enters
//! "safe mode": it stops making any paid or onchain actions (yield
//! strategy is skipped, jobs' `should_run` returns `false`). When
//! the balance recovers, the agent exits safe mode and resumes.
//!
//! # Why a separate module
//!
//! The safe-mode decision is a 2-state machine (in / out) over a
//! balance threshold. Modeling it as a separate [`SafeModeChange`]
//! enum makes the per-tick diff explicit and gives the agent's
//! audit log a clean "entered safe mode" / "exited safe mode"
//! event to record, instead of just a "safe_mode is true" boolean.
//!
//! # Decision table
//!
//! | Balance vs threshold | Current safe_mode | Result                  |
//! |----------------------|-------------------|-------------------------|
//! | `balance < threshold`| `false`           | `Enter { ... }`         |
//! | `balance < threshold`| `true`            | `NoChange { true }`     |
//! | `balance >= threshold`| `false`          | `NoChange { false }`    |
//! | `balance >= threshold`| `true`           | `Exit { ... }`          |
//!
//! Boundary: `balance == threshold` is **not** safe mode (strict
//! `<`). The plan calls for `balance < $5 → enter`, so
//! `balance == 5.0` is operational.

use std::fmt;

/// The safe-mode state change for a single tick.
///
/// Returned by [`check`]. The agent loop uses the variant to
/// (a) update `state.safe_mode`, and (b) emit a single
/// `Enter`/`Exit` log line (and, in a future iteration, a
/// Telegram alert) only on transitions — not on every tick.
#[derive(Debug, Clone, PartialEq)]
pub enum SafeModeChange {
    /// Balance dropped below threshold; safe_mode was `false`.
    /// The agent should set `safe_mode = true` and log an
    /// `Entering safe mode` line.
    Enter {
        balance: f64,
        threshold: f64,
    },
    /// Balance recovered above threshold; safe_mode was `true`.
    /// The agent should set `safe_mode = false` and log an
    /// `Exiting safe mode` line.
    Exit {
        balance: f64,
        threshold: f64,
    },
    /// No state change. The `safe_mode` field carries the
    /// current (unchanged) state for callers that want to
    /// branch on it.
    NoChange {
        safe_mode: bool,
    },
}

impl fmt::Display for SafeModeChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enter { balance, threshold } => {
                write!(f, "enter(balance={balance:.2} < threshold={threshold:.2})")
            }
            Self::Exit { balance, threshold } => {
                write!(f, "exit(balance={balance:.2} >= threshold={threshold:.2})")
            }
            Self::NoChange { safe_mode: true } => write!(f, "noop(still in safe mode)"),
            Self::NoChange { safe_mode: false } => write!(f, "noop(operational)"),
        }
    }
}

/// Compute the safe-mode change for a tick.
///
/// Pure function. `current_safe_mode` is the agent's
/// `state.safe_mode` at the start of the tick; the caller applies
/// the result to the state.
pub fn check(balance: f64, threshold: f64, current_safe_mode: bool) -> SafeModeChange {
    let should_be_safe = balance < threshold;
    match (should_be_safe, current_safe_mode) {
        (true, false) => SafeModeChange::Enter { balance, threshold },
        (false, true) => SafeModeChange::Exit { balance, threshold },
        (true, true) => SafeModeChange::NoChange { safe_mode: true },
        (false, false) => SafeModeChange::NoChange { safe_mode: false },
    }
}

/// Should the agent currently be in safe mode, given the latest
/// balance?
///
/// Equivalent to `balance < threshold`. Exists as a named
/// function for use in tests and in any code that doesn't care
/// about the state-diff (just wants the boolean).
pub fn should_enter(balance: f64, threshold: f64) -> bool {
    balance < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    // Default $5 threshold from AgentConfig.
    const THRESHOLD: f64 = 5.0;

    // --- check() — full transition table --------------------------------

    #[test]
    fn check_enters_safe_mode_when_balance_below_threshold() {
        // Operational → safe mode
        let change = check(4.99, THRESHOLD, false);
        assert_eq!(
            change,
            SafeModeChange::Enter {
                balance: 4.99,
                threshold: THRESHOLD,
            }
        );
    }

    #[test]
    fn check_enters_when_balance_is_zero() {
        // Defensive: zero balance is a strong Enter signal
        let change = check(0.0, THRESHOLD, false);
        assert!(matches!(change, SafeModeChange::Enter { .. }));
    }

    #[test]
    fn check_exits_safe_mode_when_balance_recovers() {
        // In safe mode → operational
        let change = check(10.0, THRESHOLD, true);
        assert_eq!(
            change,
            SafeModeChange::Exit {
                balance: 10.0,
                threshold: THRESHOLD,
            }
        );
    }

    #[test]
    fn check_stays_in_safe_mode_when_balance_still_low() {
        // Already in safe mode, balance still below threshold
        let change = check(2.0, THRESHOLD, true);
        assert_eq!(change, SafeModeChange::NoChange { safe_mode: true });
    }

    #[test]
    fn check_stays_out_when_balance_already_above_threshold() {
        // Operational, balance still above threshold
        let change = check(50.0, THRESHOLD, false);
        assert_eq!(change, SafeModeChange::NoChange { safe_mode: false });
    }

    // --- check() — boundary ----------------------------------------------

    #[test]
    fn check_at_threshold_is_no_change_when_outside() {
        // balance == threshold → NOT safe mode (strict <)
        let change = check(5.0, THRESHOLD, false);
        assert_eq!(change, SafeModeChange::NoChange { safe_mode: false });
    }

    #[test]
    fn check_at_threshold_exits_when_in_safe_mode() {
        // If we were in safe mode and the balance has recovered
        // to exactly the threshold, we exit.
        let change = check(5.0, THRESHOLD, true);
        assert_eq!(
            change,
            SafeModeChange::Exit {
                balance: 5.0,
                threshold: THRESHOLD,
            }
        );
    }

    #[test]
    fn check_one_cent_below_threshold_enters() {
        // The smallest possible "below threshold" value
        let change = check(4.99, THRESHOLD, false);
        assert!(matches!(change, SafeModeChange::Enter { .. }));
    }

    #[test]
    fn check_one_cent_above_threshold_is_no_change_outside() {
        let change = check(5.01, THRESHOLD, false);
        assert_eq!(change, SafeModeChange::NoChange { safe_mode: false });
    }

    // --- should_enter() -------------------------------------------------

    #[test]
    fn should_enter_matches_balance_comparison() {
        assert!(should_enter(0.0, 5.0));
        assert!(should_enter(4.99, 5.0));
        assert!(!should_enter(5.0, 5.0));
        assert!(!should_enter(5.01, 5.0));
        assert!(!should_enter(100.0, 5.0));
    }

    #[test]
    fn should_enter_handles_negative_balance() {
        // Shouldn't happen in practice (underflow guard), but the
        // function is a simple comparison and should be defensive.
        assert!(should_enter(-1.0, 5.0));
    }

    // --- Display --------------------------------------------------------

    #[test]
    fn safe_mode_change_display() {
        assert_eq!(
            SafeModeChange::Enter {
                balance: 4.0,
                threshold: 5.0
            }
            .to_string(),
            "enter(balance=4.00 < threshold=5.00)"
        );
        assert_eq!(
            SafeModeChange::Exit {
                balance: 10.0,
                threshold: 5.0
            }
            .to_string(),
            "exit(balance=10.00 >= threshold=5.00)"
        );
        assert_eq!(
            SafeModeChange::NoChange { safe_mode: true }.to_string(),
            "noop(still in safe mode)"
        );
        assert_eq!(
            SafeModeChange::NoChange { safe_mode: false }.to_string(),
            "noop(operational)"
        );
    }
}
