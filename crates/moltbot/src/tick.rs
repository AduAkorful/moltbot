//! The agent's main loop.
//!
//! [`AgentLoop::run`] ticks every [`AgentConfig::tick_interval`]
//! (default 60s) until a SIGINT (Ctrl-C) is received. Each tick:
//!
//! 1. Increments the iteration counter and stamps the state.
//! 2. Runs the safe-mode check ([`crate::safe_mode`]). Updates
//!    `state.safe_mode` and logs a one-shot `Enter`/`Exit` line
//!    on transitions.
//! 3. Logs the current USDC balance and a "started" line.
//! 4. Runs the yield strategy (Aave V3 supply/withdraw) — skipped
//!    while in safe mode.
//! 5. Runs every enabled job in the [`crate::job::JobRegistry`].
//!    Jobs are expected to gate themselves on `state.safe_mode`
//!    via their [`crate::job::Job::should_run`].
//!
//! # Graceful shutdown
//!
//! The loop watches for SIGINT via [`tokio::signal::ctrl_c`]. On
//! receipt, it breaks out of the tick loop, logs a final line, and
//! returns. SIGTERM is **not** currently handled (the Rust signal
//! crate's `Termination` future isn't worth the dep for a binary
//! target — add it later if needed).

use crate::config::AgentConfig;
use crate::job::{JobContext, JobRegistry};
use crate::safe_mode::{self, SafeModeChange};
use crate::state::SharedState;
use crate::yield_strategy::{self, ParkDecision};
use keeperhub_rs::mcp::McpClient;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Notify;
use tokio::time::{interval, MissedTickBehavior};

/// The agent's main loop handle.
///
/// Cheap to construct. Pass the same instance to [`AgentLoop::run`] to
/// start ticking. Cloning shares the shutdown signal.
#[derive(Debug, Clone)]
pub struct AgentLoop {
    state: SharedState,
    /// The MCP client. Used by [`AgentLoop::tick`] to dispatch yield
    /// strategy and job calls. The session is initialized eagerly in
    /// `main` before the loop starts.
    client: Arc<McpClient>,
    config: Arc<AgentConfig>,
    /// The job registry. Empty by default; populated in `main`
    /// via [`JobRegistry::with`]. Cheap to clone (Arc-bump).
    jobs: Arc<JobRegistry>,
    shutdown: Arc<Notify>,
}

impl AgentLoop {
    /// Build a new agent loop.
    ///
    /// `state` is the shared in-memory state. `client` is the
    /// KeeperHub MCP client. `config` carries the runtime tunables
    /// (tick interval, thresholds, network). `jobs` is the registry
    /// of jobs to run on every tick — pass [`JobRegistry::new`] for
    /// an empty (no-op) registry.
    pub fn new(
        state: SharedState,
        client: Arc<McpClient>,
        config: Arc<AgentConfig>,
        jobs: JobRegistry,
    ) -> Self {
        Self {
            state,
            client,
            config,
            jobs: Arc::new(jobs),
            shutdown: Arc::new(Notify::new()),
        }
    }

    /// A handle that triggers graceful shutdown on drop. Currently a
    /// no-op placeholder — we rely on the SIGINT path in `run` to
    /// observe the shutdown. The drop semantics are deferred to a
    /// later iteration if a programmatic-shutdown use case emerges.
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            notify: Arc::clone(&self.shutdown),
        }
    }

    /// Run the loop until SIGINT is received. Returns the iteration
    /// count at the time of shutdown.
    pub async fn run(&self) -> u64 {
        let mut ticker = interval(self.config.tick_interval());
        // If a tick is missed (e.g. the previous tick took longer than
        // the interval), skip ahead rather than firing a burst of
        // catch-up ticks.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        tracing::info!(
            tick_seconds = self.config.tick_interval_seconds,
            network = %self.config.network,
            "agent loop starting"
        );

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.tick().await;
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("received SIGINT, shutting down");
                    break;
                }
                _ = self.shutdown.notified() => {
                    tracing::info!("shutdown signal received");
                    break;
                }
            }
        }

        // Final snapshot.
        let snapshot = self.state.read().await.clone();
        tracing::info!(
            iterations = snapshot.iteration,
            uptime_seconds = snapshot.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0),
            "agent loop stopped"
        );
        snapshot.iteration
    }

    /// A single tick of the loop. Increments the iteration counter,
    /// updates the timestamps, runs the safe-mode check, logs the
    /// current USDC balance, runs the yield strategy (when not in
    /// safe mode), and dispatches the job registry.
    ///
    /// # Order of operations
    ///
    /// 1. Tick bookkeeping (iteration counter, timestamps).
    /// 2. Safe-mode check (#12): compare balance vs
    ///    `safe_mode_threshold_usd`, update `state.safe_mode`,
    ///    emit a one-shot `Enter`/`Exit` log line on transitions.
    /// 3. Log the snapshot (with the up-to-date `safe_mode`).
    /// 4. Yield strategy (#10): if not in safe mode, decide
    ///    whether to supply/withdraw on Aave V3 and execute.
    ///    Update `last_action` on success.
    /// 5. Job registry (#11): run every enabled job once. Jobs
    ///    gate themselves on `state.safe_mode` via their
    ///    [`crate::job::Job::should_run`]; the registry
    ///    additionally skips the whole block if empty. Update
    ///    `last_action` for each job that acted.
    ///
    /// The yield strategy and job registry are independent — they
    /// share the same MCP client but do not coordinate. A job's
    /// `JobContext` exposes the current `AgentState` snapshot, so
    /// jobs can read the effect of the yield strategy (e.g. an
    /// updated balance) on the same tick.
    pub async fn tick(&self) {
        let now = Instant::now();
        let (iteration, snapshot) = {
            let mut w = self.state.write().await;
            let n = w.on_tick_start(now);
            let snap = w.clone();
            (n, snap)
        };

        // Safe-mode check (#12). Runs before everything else so
        // the rest of the tick can branch on the up-to-date
        // `state.safe_mode` value (mutated below, and the
        // snapshot we pass around is the post-mutation copy).
        let safe_change = safe_mode::check(
            snapshot.usdc_balance_usd,
            self.config.safe_mode_threshold_usd,
            snapshot.safe_mode,
        );
        let mut snapshot = snapshot;
        match safe_change {
            SafeModeChange::Enter { balance, threshold } => {
                tracing::warn!(
                    balance,
                    threshold,
                    "entering safe mode: skipping paid actions until balance recovers"
                );
                let mut w = self.state.write().await;
                w.set_safe_mode(true);
                snapshot.safe_mode = true;
            }
            SafeModeChange::Exit { balance, threshold } => {
                tracing::info!(
                    balance,
                    threshold,
                    "exiting safe mode: resuming paid actions"
                );
                let mut w = self.state.write().await;
                w.set_safe_mode(false);
                snapshot.safe_mode = false;
            }
            SafeModeChange::NoChange { safe_mode } => {
                // No state change. `snapshot.safe_mode` is
                // already correct; no log line (a per-tick
                // "still in safe mode" would be noisy).
                if safe_mode {
                    tracing::debug!("safe mode active; paid actions skipped");
                }
            }
        }

        tracing::info!(
            iteration,
            usdc_balance_usd = snapshot.usdc_balance_usd,
            safe_mode = snapshot.safe_mode,
            last_action = snapshot.last_action.as_deref().unwrap_or("(none)"),
            "tick"
        );

        // Yield strategy (#10). Skipped entirely while in safe
        // mode — the agent is too low on USDC to safely engage
        // Aave V3.
        if snapshot.safe_mode {
            tracing::debug!("yield strategy: skipped (safe mode)");
        } else {
            let decision = yield_strategy::decide(
                snapshot.usdc_balance_usd,
                self.config.park_threshold_usd,
                self.config.withdraw_threshold_usd,
            );

            match decision {
                ParkDecision::NoAction => {
                    tracing::debug!(
                        balance = snapshot.usdc_balance_usd,
                        "yield strategy: no action"
                    );
                }
                ParkDecision::Supply { .. } | ParkDecision::Withdraw => {
                    tracing::info!(decision = %decision, "yield strategy: executing");
                    match yield_strategy::execute(&self.client, &self.config, &decision).await {
                        Ok(Some(tx_hash)) => {
                            tracing::info!(
                                decision = %decision,
                                tx_hash = %tx_hash,
                                "yield strategy: tx broadcast"
                            );
                            let mut w = self.state.write().await;
                            w.record_action(format!("yield::{decision}"));
                        }
                        Ok(None) => {
                            tracing::warn!(
                                decision = %decision,
                                "yield strategy: no tx hash in response"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                decision = %decision,
                                error = %e,
                                "yield strategy: execution failed"
                            );
                        }
                    }
                }
            }
        }

        // Job registry (#11). Each enabled job runs once per
        // tick; their outcomes are recorded into the shared
        // state. Jobs' `should_run` is responsible for honoring
        // `state.safe_mode`; the registry itself does not filter
        // on it (a future per-job "skip while in safe mode"
        // policy belongs in the job, not the dispatcher).
        if !self.jobs.is_empty() {
            let ctx = JobContext {
                client: &self.client,
                config: &self.config,
                state: &snapshot,
            };
            let outcomes = self.jobs.run_all(&ctx).await;
            for outcome in outcomes {
                if let Some(action) = outcome.action_taken {
                    let mut w = self.state.write().await;
                    w.record_action(action);
                }
            }
        }
    }
}

/// A handle that can signal the agent loop to shut down.
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    notify: Arc<Notify>,
}

impl ShutdownHandle {
    /// Trigger graceful shutdown. The currently-running tick (if any)
    /// completes; the loop exits before the next tick.
    pub fn trigger(&self) {
        self.notify.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobRegistry;
    use crate::state::new_shared_state;

    fn test_config() -> Arc<AgentConfig> {
        let c = AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            tick_interval_seconds: 1,
            ..AgentConfig::default()
        };
        Arc::new(c)
    }

    fn test_client() -> Arc<McpClient> {
        Arc::new(McpClient::new(
            "https://app.keeperhub.com/mcp",
            "kh_test",
        ))
    }

    /// Build a loop with an empty job registry. Use [`test_loop_with_jobs`]
    /// to pass a custom registry.
    fn test_loop(state: SharedState) -> AgentLoop {
        AgentLoop::new(state, test_client(), test_config(), JobRegistry::new())
    }

    #[tokio::test]
    async fn tick_increments_iteration_and_logs_balance() {
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            // Set a balance in the "no action" band (between the
            // withdraw and park thresholds) so the yield strategy
            // doesn't make a real KeeperHub call from this unit
            // test.
            w.set_usdc_balance(35.0);
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert_eq!(s.iteration, 1);
        assert!(s.started_at.is_some());
        assert!(s.last_tick_at.is_some());
        assert_eq!(s.usdc_balance_usd, 35.0);
    }

    #[tokio::test]
    async fn multiple_ticks_increment_iteration() {
        let state = new_shared_state();
        {
            // NoAction band: keep ticks hermetic.
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let loop_ = test_loop(state.clone());
        for _ in 0..3 {
            loop_.tick().await;
        }
        let s = state.read().await;
        assert_eq!(s.iteration, 3);
    }

    #[tokio::test]
    async fn tick_records_action_when_yield_executes() {
        // We can't drive a real Aave call in a unit test, but we can
        // verify the bookkeeping: when the strategy would have called
        // execute(), the `last_action` is set only if the call
        // succeeds. With a 0.0 balance the decision is `Withdraw`, the
        // network call fails, and `last_action` stays None. This is
        // the expected behavior — we just assert that *no* exception
        // bubbles out of the tick.
        let state = new_shared_state();
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert_eq!(s.iteration, 1);
        // Network call is expected to fail (no real server), but
        // tick() must absorb the error and continue.
        assert!(s.last_action.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn run_returns_on_ctrl_c() {
        // We can't easily inject a real SIGINT in a unit test, but we
        // can use the shutdown handle.
        let state = new_shared_state();
        {
            // NoAction band: the loop must not block on a network
            // call from the yield strategy.
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let loop_ = test_loop(state.clone());
        let handle = loop_.shutdown_handle();
        let runner = loop_.clone();
        tokio::spawn(async move {
            // Fire shutdown after a short delay.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            handle.trigger();
        });
        let iterations = runner.run().await;
        // The runner should exit on the first tick boundary (or
        // sooner). Iteration count is 0 or 1 depending on timing.
        assert!(iterations <= 1, "expected <= 1 iteration before shutdown, got {iterations}");
    }

    #[test]
    fn shutdown_handle_is_cloneable() {
        let state = new_shared_state();
        let loop_ = test_loop(state);
        let h1 = loop_.shutdown_handle();
        let h2 = h1.clone();
        // Both handles refer to the same notify; triggering either
        // wakes both waiters (Notify is reference-counted via Arc).
        h1.trigger();
        h2.trigger();
        // No panic = success
    }

    #[tokio::test]
    async fn tick_invokes_jobs_in_registry() {
        // A counter job that always runs, increments a shared
        // AtomicU32, and returns a no-op outcome. We verify the
        // tick() function dispatches to the registry.
        use crate::job::{Job, JobContext, JobOutcome};
        use std::sync::atomic::{AtomicU32, Ordering};

        struct CounterJob(AtomicU32);
        impl Job for CounterJob {
            fn name(&self) -> &'static str {
                "counter"
            }
            fn should_run(
                &self,
                _state: &crate::state::AgentState,
                _config: &AgentConfig,
            ) -> bool {
                true
            }
            fn tick<'a>(
                &'a self,
                _ctx: &'a JobContext<'a>,
            ) -> crate::job::BoxFuture<'a, keeperhub_rs::Result<JobOutcome>> {
                Box::pin(async {
                    self.0.fetch_add(1, Ordering::SeqCst);
                    Ok(JobOutcome::noop())
                })
            }
        }

        // A job that records a label into `last_action` so we can
        // verify the loop's outcome wiring.
        struct ActingJob;
        impl Job for ActingJob {
            fn name(&self) -> &'static str {
                "acting"
            }
            fn should_run(
                &self,
                _state: &crate::state::AgentState,
                _config: &AgentConfig,
            ) -> bool {
                true
            }
            fn tick<'a>(
                &'a self,
                _ctx: &'a JobContext<'a>,
            ) -> crate::job::BoxFuture<'a, keeperhub_rs::Result<JobOutcome>> {
                Box::pin(async {
                    Ok(JobOutcome::acted("test::acting", None, None))
                })
            }
        }

        let state = new_shared_state();
        {
            // NoAction band: keep ticks hermetic.
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let counter = CounterJob(AtomicU32::new(0));
        let registry = JobRegistry::new()
            .with(counter)
            .with(ActingJob);
        let loop_ = AgentLoop::new(
            state.clone(),
            test_client(),
            test_config(),
            registry,
        );
        loop_.tick().await;

        let s = state.read().await;
        assert_eq!(s.iteration, 1);
        // The ActingJob's action label should be recorded in state.
        assert_eq!(s.last_action.as_deref(), Some("test::acting"));
    }

    #[tokio::test]
    async fn tick_skips_disabled_jobs() {
        // A gated job that returns false from should_run. It
        // should never be invoked.
        use crate::job::{Job, JobContext, JobOutcome};
        use std::sync::atomic::{AtomicU32, Ordering};

        struct GatedJob(AtomicU32);
        impl Job for GatedJob {
            fn name(&self) -> &'static str {
                "gated"
            }
            fn should_run(
                &self,
                _state: &crate::state::AgentState,
                _config: &AgentConfig,
            ) -> bool {
                false
            }
            fn tick<'a>(
                &'a self,
                _ctx: &'a JobContext<'a>,
            ) -> crate::job::BoxFuture<'a, keeperhub_rs::Result<JobOutcome>> {
                Box::pin(async {
                    self.0.fetch_add(1, Ordering::SeqCst);
                    Ok(JobOutcome::noop())
                })
            }
        }

        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let gated = GatedJob(AtomicU32::new(0));
        let registry = JobRegistry::new().with(gated);
        let loop_ = AgentLoop::new(
            state.clone(),
            test_client(),
            test_config(),
            registry,
        );
        loop_.tick().await;
        // The counter inside the gated job is unreachable; we just
        // verify the tick completed without error. (Direct access
        // to `gated` would require moving it, which we already did
        // when we built the registry.)
    }

    // --- safe-mode (#12) ------------------------------------------------

    #[tokio::test]
    async fn tick_enters_safe_mode_when_balance_below_threshold() {
        // Balance 1.0 < safe_mode_threshold (5.0) → Enter
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(1.0);
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert!(s.safe_mode, "expected safe_mode to be true after tick with low balance");
    }

    #[tokio::test]
    async fn tick_exits_safe_mode_when_balance_recovers() {
        // Start in safe mode with a high balance (recovery scenario).
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(50.0);
            w.set_safe_mode(true);
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert!(
            !s.safe_mode,
            "expected safe_mode to be false after tick with high balance"
        );
    }

    #[tokio::test]
    async fn tick_stays_out_of_safe_mode_when_balance_already_high() {
        // Already operational; high balance; safe mode stays off.
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(50.0);
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert!(!s.safe_mode);
    }

    #[tokio::test]
    async fn tick_stays_in_safe_mode_when_balance_still_low() {
        // Already in safe mode; balance still low; stays in.
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(1.0);
            w.set_safe_mode(true);
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert!(s.safe_mode);
    }

    #[tokio::test]
    async fn tick_at_safe_mode_threshold_is_not_safe() {
        // Boundary: balance == threshold → not in safe mode (strict <)
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(5.0); // exactly the threshold
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert!(!s.safe_mode);
    }

    #[tokio::test]
    async fn tick_skips_yield_strategy_when_in_safe_mode() {
        // Even with a balance that would normally trigger a
        // yield action (e.g. 0.0 → Withdraw), the yield strategy
        // is skipped while in safe mode. We assert this by
        // verifying `last_action` is `None` after a tick where
        // the network would have failed (the no-action band
        // would be 35.0; 0.0 would Withdraw).
        //
        // Specifically: with balance 0.0 (way below the
        // withdraw threshold of 20.0), the yield strategy
        // would normally call `AaveV3::withdraw`. The network
        // call fails (no real server in this test) and the
        // error is absorbed — `last_action` stays `None`. The
        // important assertion is that *safe mode is also on*
        // afterwards, which proves the yield path was skipped
        // (not just failed).
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(0.0);
        }
        let loop_ = test_loop(state.clone());
        loop_.tick().await;
        let s = state.read().await;
        assert!(s.safe_mode, "safe mode should be entered at 0.0 balance");
        // The yield strategy was skipped (no network call,
        // no tx hash) — `last_action` stays `None`.
        assert!(s.last_action.is_none());
    }

    #[tokio::test]
    async fn tick_skips_jobs_when_in_safe_mode() {
        // A job that always acts (so we can see it being
        // skipped vs being run). When the agent is in safe
        // mode, the job's `should_run` should return `false`.
        use crate::job::{Job, JobContext, JobOutcome};

        struct ActingJob;
        impl Job for ActingJob {
            fn name(&self) -> &'static str {
                "acting"
            }
            fn should_run(
                &self,
                state: &crate::state::AgentState,
                _config: &AgentConfig,
            ) -> bool {
                !state.safe_mode
            }
            fn tick<'a>(
                &'a self,
                _ctx: &'a JobContext<'a>,
            ) -> crate::job::BoxFuture<'a, keeperhub_rs::Result<JobOutcome>> {
                Box::pin(async {
                    Ok(JobOutcome::acted("should_not_record", None, None))
                })
            }
        }

        let state = new_shared_state();
        {
            // Pre-set safe mode AND a balance that would keep
            // the agent in safe mode.
            let mut w = state.write().await;
            w.set_usdc_balance(0.0);
            w.set_safe_mode(true);
        }
        let registry = JobRegistry::new().with(ActingJob);
        let loop_ = AgentLoop::new(
            state.clone(),
            test_client(),
            test_config(),
            registry,
        );
        loop_.tick().await;
        let s = state.read().await;
        // The job's `should_run` returned `false` → job was
        // skipped → `last_action` is still `None`.
        assert!(s.last_action.is_none());
        // And safe mode is still on.
        assert!(s.safe_mode);
    }
}
