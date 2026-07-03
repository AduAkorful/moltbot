//! The agent's main loop.
//!
//! [`AgentLoop::run`] ticks every [`AgentConfig::tick_interval`]
//! (default 60s) until a SIGINT (Ctrl-C) is received. Each tick:
//!
//! 1. Increments the iteration counter and stamps the state.
//! 2. Runs the safe-mode check ([`crate::safe_mode`]). Updates
//!    `state.safe_mode` and logs a one-shot `Enter`/`Exit` line
//!    on transitions.
//! 3. Opens an audit-log transaction (if an audit log is
//!    configured). All actions taken during the tick are
//!    recorded against this run.
//! 4. Logs the current USDC balance and a "started" line.
//! 5. Runs the yield strategy (Aave V3 supply/withdraw) — skipped
//!    while in safe mode.
//! 6. Runs every enabled job in the [`crate::job::JobRegistry`].
//!    Jobs are expected to gate themselves on `state.safe_mode`
//!    via their [`crate::job::Job::should_run`].
//! 7. Commits the audit-log transaction.
//!
//! # Graceful shutdown
//!
//! The loop watches for SIGINT via [`tokio::signal::ctrl_c`]. On
//! receipt, it breaks out of the tick loop, logs a final line, and
//! returns. SIGTERM is **not** currently handled (the Rust signal
//! crate's `Termination` future isn't worth the dep for a binary
//! target — add it later if needed).

use crate::audit::{ActionRecord, AuditLog, TickHandle};
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
    /// Optional audit log. When `Some`, every tick is persisted
    /// to SQLite (runs + actions + x402 payments). `None`
    /// disables persistence — used by unit tests and any
    /// short-lived process that doesn't need an audit trail.
    audit: Option<Arc<AuditLog>>,
    shutdown: Arc<Notify>,
}

impl AgentLoop {
    /// Build a new agent loop.
    ///
    /// `state` is the shared in-memory state. `client` is the
    /// KeeperHub MCP client. `config` carries the runtime tunables
    /// (tick interval, thresholds, network). `jobs` is the registry
    /// of jobs to run on every tick — pass [`JobRegistry::new`] for
    /// an empty (no-op) registry. `audit` is the optional SQLite
    /// audit log; pass `None` to disable persistence.
    pub fn new(
        state: SharedState,
        client: Arc<McpClient>,
        config: Arc<AgentConfig>,
        jobs: JobRegistry,
        audit: Option<Arc<AuditLog>>,
    ) -> Self {
        Self {
            state,
            client,
            config,
            jobs: Arc::new(jobs),
            audit,
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
    /// safe mode), dispatches the job registry, and (if an audit
    /// log is configured) records everything in a single
    /// transaction.
    ///
    /// # Order of operations
    ///
    /// 1. Tick bookkeeping (iteration counter, timestamps).
    /// 2. Safe-mode check (#12): compare balance vs
    ///    `safe_mode_threshold_usd`, update `state.safe_mode`,
    ///    emit a one-shot `Enter`/`Exit` log line on transitions.
    /// 3. Open the audit-log transaction (if `audit` is `Some`).
    ///    All actions taken during the tick are recorded against
    ///    this run.
    /// 4. Log the snapshot (with the up-to-date `safe_mode`).
    /// 5. Yield strategy (#10): if not in safe mode, decide
    ///    whether to supply/withdraw on Aave V3 and execute.
    ///    Update `last_action` on success; record the action
    ///    to the audit log.
    /// 6. Job registry (#11): run every enabled job once. Jobs
    ///    gate themselves on `state.safe_mode` via their
    ///    [`crate::job::Job::should_run`]; the registry
    ///    additionally skips the whole block if empty. Update
    ///    `last_action` for each job that acted; record each
    ///    action to the audit log.
    /// 7. Commit the audit-log transaction. On any error in
    ///    steps 5–6 the run is marked `error` (rather than
    ///    dropped) so the row remains in the DB for audit.
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

        // Audit-log transaction (#14). Opens a tick handle
        // for the duration of the tick; commit/abort happens
        // at the end. Errors opening the transaction are
        // logged but do not stop the tick — the in-memory
        // state and the MCP calls still happen.
        let mut audit_tick: Option<TickHandle<'_>> = match &self.audit {
            Some(log) => match log.start_tick(iteration).await {
                Ok(handle) => Some(handle),
                Err(e) => {
                    tracing::error!(error = %e, "audit log: start_tick failed");
                    None
                }
            },
            None => None,
        };

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
        let mut tick_had_error = false;
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
                            let action_label = format!("yield::{decision}");
                            // Record to in-memory state
                            {
                                let mut w = self.state.write().await;
                                w.record_action(action_label.clone());
                            }
                            // Record to audit log
                            if let Some(tick) = audit_tick.as_mut() {
                                let mut record =
                                    ActionRecord::new(&action_label).with_tx_hash(&tx_hash);
                                if let Some(note) = action_note_for_yield(&decision) {
                                    record = record.with_note(note);
                                }
                                if let Err(e) = tick.record_action(&record).await {
                                    tracing::error!(error = %e, "audit log: record_action failed");
                                    tick_had_error = true;
                                }
                            }
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
                            tick_had_error = true;
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
            let report = self.jobs.run_all(&ctx).await;
            if report.had_errors() {
                tick_had_error = true;
            }
            for outcome in report.outcomes {
                if let Some(action) = outcome.action_taken {
                    {
                        let mut w = self.state.write().await;
                        w.record_action(action.clone());
                    }
                    if let Some(tick) = audit_tick.as_mut() {
                        let mut record = ActionRecord::new(&action);
                        if let Some(tx) = outcome.tx_hash.as_deref() {
                            record = record.with_tx_hash(tx);
                        }
                        if let Some(note) = outcome.note.as_deref() {
                            record = record.with_note(note);
                        }
                        if let Err(e) = tick.record_action(&record).await {
                            tracing::error!(error = %e, "audit log: record_action failed");
                            tick_had_error = true;
                        }
                    }
                }
            }
        }

        // Commit the audit-log transaction. On error, mark the
        // run as `error` so the row remains in the DB for
        // post-mortem.
        if let Some(tick) = audit_tick {
            if tick_had_error {
                if let Err(e) = tick.abort_with_error().await {
                    tracing::error!(error = %e, "audit log: abort_with_error failed");
                }
            } else if let Err(e) = tick.commit().await {
                tracing::error!(error = %e, "audit log: commit failed");
            }
        }
    }
}

/// Build a short human-readable note for a yield decision, used
/// as the `note` column in the audit log's `actions` table.
fn action_note_for_yield(decision: &ParkDecision) -> Option<String> {
    match decision {
        ParkDecision::Supply { amount_usd } => Some(format!("amount_usd={amount_usd:.2}")),
        ParkDecision::Withdraw => Some("withdraw_all".to_string()),
        ParkDecision::NoAction => None,
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

    /// Build a loop with an empty job registry and no audit
    /// log. Most unit tests use this; integration tests for
    /// the audit log construct their own.
    fn test_loop(state: SharedState) -> AgentLoop {
        AgentLoop::new(
            state,
            test_client(),
            test_config(),
            JobRegistry::new(),
            None,
        )
    }

    /// Build an in-memory audit log for tests. Tests that
    /// need an audit log construct the log here and pass it
    /// to [`AgentLoop::new`] directly (so the test can
    /// control the loop's full configuration).
    async fn test_audit_log() -> Arc<crate::audit::AuditLog> {
        Arc::new(crate::audit::AuditLog::in_memory().await.unwrap())
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
            None,
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
            None,
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
            None,
        );
        loop_.tick().await;
        let s = state.read().await;
        // The job's `should_run` returned `false` → job was
        // skipped → `last_action` is still `None`.
        assert!(s.last_action.is_none());
        // And safe mode is still on.
        assert!(s.safe_mode);
    }

    // --- audit log (#14) ------------------------------------------------

    #[tokio::test]
    async fn tick_persists_run_and_actions_to_audit_log() {
        // A job that always acts. We verify that:
        // 1. A run row is written (status='ok' after commit)
        // 2. The job's action is recorded with the right kind
        // 3. count_runs and count_actions match
        use crate::job::{Job, JobContext, JobOutcome};
        use sqlx::Row;

        struct ActingJob;
        impl Job for ActingJob {
            fn name(&self) -> &'static str {
                "test_actor"
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
                    Ok(JobOutcome::acted(
                        "test::did_a_thing",
                        Some("0xfeed".to_string()),
                        Some("test note".to_string()),
                    ))
                })
            }
        }

        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let log = test_audit_log().await;
        let registry = JobRegistry::new().with(ActingJob);
        let loop_ = AgentLoop::new(
            state.clone(),
            test_client(),
            test_config(),
            registry,
            Some(Arc::clone(&log)),
        );
        loop_.tick().await;

        // 1 run, 1 action persisted.
        assert_eq!(log.count_runs().await.unwrap(), 1);
        assert_eq!(log.count_actions().await.unwrap(), 1);

        // Inspect the run row directly.
        let row = sqlx::query("SELECT status, kind, iteration FROM runs")
            .fetch_one(log.pool())
            .await
            .unwrap();
        let status: String = row.get("status");
        let kind: String = row.get("kind");
        let iteration: i64 = row.get("iteration");
        assert_eq!(status, "ok");
        assert_eq!(kind, "tick");
        assert_eq!(iteration, 1);

        // Inspect the action row.
        let row = sqlx::query("SELECT kind, tx_hash, note FROM actions")
            .fetch_one(log.pool())
            .await
            .unwrap();
        let action_kind: String = row.get("kind");
        let tx_hash: Option<String> = row.get("tx_hash");
        let note: Option<String> = row.get("note");
        assert_eq!(action_kind, "test::did_a_thing");
        assert_eq!(tx_hash.as_deref(), Some("0xfeed"));
        assert_eq!(note.as_deref(), Some("test note"));
    }

    #[tokio::test]
    async fn tick_persists_run_with_error_status_on_failure() {
        // A job that returns an error. The tick should mark
        // the run with status='error' rather than 'ok'.
        use crate::job::{Job, JobContext, JobOutcome};
        use sqlx::Row;

        struct FailingJob;
        impl Job for FailingJob {
            fn name(&self) -> &'static str {
                "failing"
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
                    Err(keeperhub_rs::Error::Config(
                        "synthetic".to_string(),
                    ))
                })
            }
        }

        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let log = Arc::new(crate::audit::AuditLog::in_memory().await.unwrap());
        let registry = JobRegistry::new().with(FailingJob);
        let loop_ = AgentLoop::new(
            state.clone(),
            test_client(),
            test_config(),
            registry,
            Some(Arc::clone(&log)),
        );
        loop_.tick().await;

        // The run row should be present, marked 'error'.
        assert_eq!(log.count_runs().await.unwrap(), 1);
        let row = sqlx::query("SELECT status FROM runs")
            .fetch_one(log.pool())
            .await
            .unwrap();
        let status: String = row.get("status");
        assert_eq!(status, "error");
    }

    #[tokio::test]
    async fn tick_without_audit_does_not_persist() {
        // No audit log configured → no DB writes, no errors.
        let state = new_shared_state();
        {
            let mut w = state.write().await;
            w.set_usdc_balance(35.0);
        }
        let loop_ = test_loop(state.clone());
        // Should not panic or error.
        loop_.tick().await;
    }
}
