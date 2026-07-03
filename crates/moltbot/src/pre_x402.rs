//! Pre-x402 balance check — defense in depth.
//!
//! Before the agent issues any paid KeeperHub call (e.g. an
//! Aave supply/withdraw that goes through the x402 payment
//! flow), it consults [`should_skip`] to confirm the wallet
//! has enough USDC to cover the call's price tag. If not, the
//! call is skipped and a debug line is logged — but the
//! agent does **not** enter safe mode (that's reserved for
//! the much lower $5 floor; see [`crate::safe_mode`]).
//!
//! # Why a separate module
//!
//! The check is conceptually a guard rail, not a strategy.
//! Keeping it out of `yield_strategy` (which decides whether
//! to *act*) and out of `safe_mode` (which decides whether to
//! operate at all) makes the layering explicit:
//!
//! - **safe_mode** — "do we have enough to keep running?"
//! - **pre_x402** — "do we have enough for *this* call?"
//! - **yield_strategy** — "should we act this tick?"
//!
//! # Decision
//!
//! [`should_skip`] returns `true` when the wallet's USDC
//! balance is below the configured per-call cap. The
//! comparison is strict (`<`), so `balance == cap` proceeds.
//!
//! The default cap is $0.10 (see
//! [`AgentConfig::max_x402_payment_usd`]); typical x402-paid
//! KeeperHub calls run $0.01–$0.05, so the default leaves
//! 2–10× headroom.

/// Should the agent skip a paid call given the current
/// wallet balance and the per-call x402 cap?
///
/// Pure function. Returns `true` when `balance < cap`,
/// `false` otherwise. Negative balances are treated as
/// "definitely skip" (defensive: shouldn't happen in
/// practice, but `f64` allows it).
pub fn should_skip(balance_usd: f64, max_x402_payment_usd: f64) -> bool {
    balance_usd < max_x402_payment_usd
}

/// A human-readable reason for the skip, suitable for the
/// audit-log note column. Returns `None` when the call
/// should not be skipped.
pub fn skip_reason(
    balance_usd: f64,
    max_x402_payment_usd: f64,
) -> Option<String> {
    if should_skip(balance_usd, max_x402_payment_usd) {
        Some(format!(
            "balance=${:.4} < max_x402_payment=${:.4}",
            balance_usd, max_x402_payment_usd
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Default cap.
    const CAP: f64 = 0.10;

    #[test]
    fn should_skip_when_balance_below_cap() {
        // 1 cent below the cap → skip
        assert!(should_skip(0.09, CAP));
    }

    #[test]
    fn should_skip_when_balance_is_zero() {
        assert!(should_skip(0.0, CAP));
    }

    #[test]
    fn should_skip_when_balance_is_negative() {
        // Defensive: f64 allows this; treat as skip.
        assert!(should_skip(-1.0, CAP));
    }

    #[test]
    fn should_not_skip_when_balance_equals_cap() {
        // Boundary: balance == cap → proceed (strict <).
        assert!(!should_skip(CAP, CAP));
    }

    #[test]
    fn should_not_skip_when_balance_just_above_cap() {
        // 1 cent above → proceed
        assert!(!should_skip(0.11, CAP));
    }

    #[test]
    fn should_not_skip_when_balance_way_above_cap() {
        assert!(!should_skip(50.0, CAP));
    }

    #[test]
    fn should_respect_custom_cap() {
        // Higher cap → stricter (more skip) — we have less
        // than the cap, so skip.
        assert!(should_skip(0.20, 0.25));
        // Lower cap → more permissive — we have more than
        // the cap, so proceed.
        assert!(!should_skip(0.20, 0.15));
    }

    #[test]
    fn should_not_skip_when_cap_is_zero_and_balance_is_zero() {
        // Edge: 0 < 0 is false, so balance == 0 with cap == 0
        // does NOT skip. The default cap is $0.10, so this is
        // only relevant for misconfigured caps.
        assert!(!should_skip(0.0, 0.0));
    }

    #[test]
    fn should_skip_when_cap_is_huge_and_balance_is_small() {
        // With a huge cap (e.g. $1,000), a $0.05 balance is
        // "not enough" — we skip. (Misconfigured cap, but the
        // function is honest: the cap is a hard floor.)
        assert!(should_skip(0.05, 1_000.0));
    }

    // --- skip_reason() ---------------------------------------------------

    #[test]
    fn skip_reason_returns_none_when_not_skipping() {
        assert_eq!(skip_reason(50.0, CAP), None);
        assert_eq!(skip_reason(CAP, CAP), None); // boundary
    }

    #[test]
    fn skip_reason_returns_some_with_balance_and_cap() {
        let r = skip_reason(0.05, CAP).unwrap();
        assert!(r.contains("balance=$0.0500"));
        assert!(r.contains("max_x402_payment=$0.1000"));
    }

    #[test]
    fn skip_reason_includes_negative_balance() {
        let r = skip_reason(-1.0, CAP).unwrap();
        // The Display for f64 puts the sign before the `$`.
        assert!(r.contains("balance=$-1.0000"));
    }
}
