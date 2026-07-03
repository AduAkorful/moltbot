//! In-memory agent state.
//!
//! [`AgentState`] is the live runtime state of the agent loop: the
//! current USDC balance, iteration counter, last-action log, and the
//! safe-mode flag. It's wrapped in an [`Arc<RwLock>`] and shared
//! between the loop (writer) and any future HTTP dashboard (reader).
//!
//! # Concurrency model
//!
//! The state is shared via [`SharedState`]. The agent loop holds the
//! write lock for the duration of a single tick (microseconds); any
//! future dashboard reads via an async lock should batch reads into a
//! snapshot to avoid contention.
//!
//! The "actor" of a single field — for example, "the keeperhub
//! response just set `usdc_balance`" — is a method on [`AgentState`]
//! that takes `&mut self`. The caller is responsible for acquiring the
//! write lock and applying the method.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// The agent's mutable in-memory state.
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    /// Monotonic iteration counter. Starts at 0; incremented at
    /// the start of every successful tick.
    pub iteration: u64,

    /// When the agent process started. Set on first tick, never
    /// updated.
    pub started_at: Option<Instant>,

    /// When the most recent tick started. `None` until the first tick.
    pub last_tick_at: Option<Instant>,

    /// Liquid USDC in the wallet (the spendable balance). The
    /// pre-x402 check uses this; after a successful
    /// `AaveV3::supply`, it decreases by the supplied amount;
    /// after `AaveV3::withdraw`, it increases.
    pub usdc_balance_usd: f64,

    /// USDC-equivalent value parked in Aave V3 (aTokens). Starts
    /// at 0; the agent tracks this itself (no onchain read in
    /// this iteration) and updates it on every successful
    /// supply/withdraw.
    pub aave_balance_usd: f64,

    /// The last action the agent took, as a human-readable string
    /// (e.g. `"aave_v3::supply"`, `"morpho::check_hf"`). `None` until
    /// the first action.
    pub last_action: Option<String>,

    /// Whether the agent is in safe mode (skips paid actions and
    /// onchain txs). Toggled by the safe-mode check on every tick.
    pub safe_mode: bool,
}

impl AgentState {
    /// Create a fresh, empty state. All fields are `None` / `0` /
    /// `false`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the start of a tick: increment `iteration`, set
    /// `started_at` once, set `last_tick_at` to now. Returns the new
    /// iteration number for logging.
    pub fn on_tick_start(&mut self, now: Instant) -> u64 {
        self.iteration = self.iteration.saturating_add(1);
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
        self.last_tick_at = Some(now);
        self.iteration
    }

    /// Set the USDC balance. Call this after every balance read.
    pub fn set_usdc_balance(&mut self, balance_usd: f64) {
        self.usdc_balance_usd = balance_usd;
    }

    /// Total USDC-equivalent assets: liquid + parked. Used by
    /// the yield strategy and the safe-mode check.
    pub fn total_assets_usd(&self) -> f64 {
        self.usdc_balance_usd + self.aave_balance_usd
    }

    /// Record a successful `AaveV3::supply` of `amount_usd`.
    /// Decreases the wallet's liquid USDC and increases the
    /// tracked Aave balance by the same amount.
    ///
    /// Clamps the wallet balance at 0 (defensive: small
    /// rounding from `usd_to_usdc_smallest_unit`'s floor
    /// could otherwise leave a negative).
    pub fn record_supply(&mut self, amount_usd: f64) {
        self.usdc_balance_usd = (self.usdc_balance_usd - amount_usd).max(0.0);
        self.aave_balance_usd += amount_usd;
    }

    /// Record a successful `AaveV3::withdraw` of `amount_usd`.
    /// Increases the wallet's liquid USDC and decreases the
    /// tracked Aave balance by the same amount. Clamps the
    /// Aave balance at 0 (defensive).
    pub fn record_withdraw(&mut self, amount_usd: f64) {
        self.aave_balance_usd = (self.aave_balance_usd - amount_usd).max(0.0);
        self.usdc_balance_usd += amount_usd;
    }

    /// Record an action that was just taken. `action` is a
    /// human-readable label (e.g. `"aave_v3::supply"`).
    pub fn record_action(&mut self, action: impl Into<String>) {
        self.last_action = Some(action.into());
    }

    /// Set the safe-mode flag.
    pub fn set_safe_mode(&mut self, safe: bool) {
        self.safe_mode = safe;
    }
}

/// Thread-safe handle to the shared [`AgentState`].
///
/// Cloning a `SharedState` is cheap (just an `Arc` bump). The inner
/// lock serializes access.
pub type SharedState = Arc<RwLock<AgentState>>;

/// Construct a fresh [`SharedState`] wrapping a default [`AgentState`].
pub fn new_shared_state() -> SharedState {
    Arc::new(RwLock::new(AgentState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let s = AgentState::new();
        assert_eq!(s.iteration, 0);
        assert!(s.started_at.is_none());
        assert!(s.last_tick_at.is_none());
        assert_eq!(s.usdc_balance_usd, 0.0);
        assert_eq!(s.aave_balance_usd, 0.0);
        assert_eq!(s.total_assets_usd(), 0.0);
        assert!(s.last_action.is_none());
        assert!(!s.safe_mode);
    }

    #[test]
    fn on_tick_start_increments_iteration_and_sets_timestamps() {
        let mut s = AgentState::new();
        let t0 = Instant::now();
        let n1 = s.on_tick_start(t0);
        assert_eq!(n1, 1);
        assert_eq!(s.iteration, 1);
        assert_eq!(s.started_at, Some(t0));
        assert_eq!(s.last_tick_at, Some(t0));

        // Second tick: started_at unchanged, last_tick_at updated.
        let t1 = t0 + std::time::Duration::from_secs(60);
        let n2 = s.on_tick_start(t1);
        assert_eq!(n2, 2);
        assert_eq!(s.iteration, 2);
        assert_eq!(s.started_at, Some(t0));
        assert_eq!(s.last_tick_at, Some(t1));
    }

    #[test]
    fn iteration_does_not_overflow_at_u64_max() {
        let mut s = AgentState {
            iteration: u64::MAX,
            ..AgentState::new()
        };
        let n = s.on_tick_start(Instant::now());
        // saturating_add caps at u64::MAX rather than panicking
        assert_eq!(n, u64::MAX);
        assert_eq!(s.iteration, u64::MAX);
    }

    #[test]
    fn set_usdc_balance_stores_value() {
        let mut s = AgentState::new();
        s.set_usdc_balance(42.5);
        assert_eq!(s.usdc_balance_usd, 42.5);
    }

    #[test]
    fn total_assets_usd_sums_wallet_and_aave() {
        let mut s = AgentState::new();
        s.set_usdc_balance(40.0);
        s.aave_balance_usd = 20.0;
        assert_eq!(s.total_assets_usd(), 60.0);
    }

    #[test]
    fn record_supply_moves_usdc_to_aave() {
        let mut s = AgentState::new();
        s.set_usdc_balance(50.0);
        s.record_supply(50.0);
        assert_eq!(s.usdc_balance_usd, 0.0);
        assert_eq!(s.aave_balance_usd, 50.0);
        // Total preserved.
        assert_eq!(s.total_assets_usd(), 50.0);
    }

    #[test]
    fn record_supply_clamps_wallet_at_zero() {
        // Defensive: rounding from usd_to_usdc_smallest_unit's
        // floor could leave a tiny negative. We clamp at 0
        // rather than record a negative wallet balance.
        let mut s = AgentState::new();
        s.set_usdc_balance(50.0);
        s.record_supply(50.01); // slight over-supply
        assert_eq!(s.usdc_balance_usd, 0.0);
        assert_eq!(s.aave_balance_usd, 50.01);
    }

    #[test]
    fn record_withdraw_moves_aave_to_usdc() {
        let mut s = AgentState::new();
        s.set_usdc_balance(0.0);
        s.aave_balance_usd = 60.0;
        s.record_withdraw(60.0);
        assert_eq!(s.usdc_balance_usd, 60.0);
        assert_eq!(s.aave_balance_usd, 0.0);
        assert_eq!(s.total_assets_usd(), 60.0);
    }

    #[test]
    fn record_withdraw_clamps_aave_at_zero() {
        let mut s = AgentState::new();
        s.aave_balance_usd = 60.0;
        s.record_withdraw(60.01); // slight over-withdraw
        assert_eq!(s.usdc_balance_usd, 60.01);
        assert_eq!(s.aave_balance_usd, 0.0);
    }

    #[test]
    fn supply_then_withdraw_round_trip_preserves_total() {
        let mut s = AgentState::new();
        s.set_usdc_balance(80.0);
        s.record_supply(80.0);
        s.record_withdraw(80.0);
        // Wallet is back to 80, Aave is 0, total is 80.
        assert_eq!(s.usdc_balance_usd, 80.0);
        assert_eq!(s.aave_balance_usd, 0.0);
        assert_eq!(s.total_assets_usd(), 80.0);
    }

    #[test]
    fn record_action_overwrites_previous() {
        let mut s = AgentState::new();
        s.record_action("aave_v3::supply");
        assert_eq!(s.last_action.as_deref(), Some("aave_v3::supply"));
        s.record_action("morpho::check_hf");
        assert_eq!(s.last_action.as_deref(), Some("morpho::check_hf"));
    }

    #[test]
    fn set_safe_mode_toggles() {
        let mut s = AgentState::new();
        assert!(!s.safe_mode);
        s.set_safe_mode(true);
        assert!(s.safe_mode);
        s.set_safe_mode(false);
        assert!(!s.safe_mode);
    }

    #[tokio::test]
    async fn new_shared_state_is_arc_rwlock_default() {
        let shared = new_shared_state();
        // Default AgentState
        let guard = shared.read().await;
        assert_eq!(guard.iteration, 0);
        assert!(!guard.safe_mode);
    }

    #[tokio::test]
    async fn shared_state_is_clonable_and_shared() {
        let s1 = new_shared_state();
        let s2 = Arc::clone(&s1);
        {
            let mut w = s1.write().await;
            w.set_usdc_balance(99.0);
        }
        // s2 sees the same value
        let guard = s2.read().await;
        assert_eq!(guard.usdc_balance_usd, 99.0);
    }
}
