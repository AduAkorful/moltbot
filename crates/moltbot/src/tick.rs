//! The agent's main loop.
//!
//! [`AgentLoop::run`] ticks every [`AgentConfig::tick_interval`]
//! (default 60s) until a SIGINT (Ctrl-C) is received. Each tick:
//!
//! 1. Increments the iteration counter and stamps the state.
//! 2. Logs the current USDC balance and a "started" line.
//! 3. *(future: yield strategy, Morpho job, safe-mode check)*
//!
//! For the skeleton iteration (#9), the loop only logs. Yield (#10),
//! Morpho (#11), and safe-mode (#12) are layered on top in
//! subsequent iterations.
//!
//! # Graceful shutdown
//!
//! The loop watches for SIGINT via [`tokio::signal::ctrl_c`]. On
//! receipt, it breaks out of the tick loop, logs a final line, and
//! returns. SIGTERM is **not** currently handled (the Rust signal
//! crate's `Termination` future isn't worth the dep for a binary
//! target — add it later if needed).

use crate::config::AgentConfig;
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
    /// strategy calls (#10). The session is initialized eagerly in
    /// `main` before the loop starts.
    client: Arc<McpClient>,
    config: Arc<AgentConfig>,
    shutdown: Arc<Notify>,
}

impl AgentLoop {
    /// Build a new agent loop.
    ///
    /// `state` is the shared in-memory state. `client` is the
    /// KeeperHub MCP client. `config` carries the runtime tunables
    /// (tick interval, thresholds, network).
    pub fn new(state: SharedState, client: Arc<McpClient>, config: Arc<AgentConfig>) -> Self {
        Self {
            state,
            client,
            config,
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
    /// updates the timestamps, logs the current USDC balance, and
    /// runs the yield strategy.
    ///
    /// # Order of operations
    ///
    /// 1. Tick bookkeeping (iteration counter, timestamps).
    /// 2. Log the snapshot.
    /// 3. If `safe_mode` is set, skip the yield strategy (the safe
    ///    mode itself is managed by iteration #12; until then this
    ///    is a no-op).
    /// 4. Run the yield strategy: decide whether to supply or
    ///    withdraw from Aave V3, and execute if so. Update
    ///    `last_action` and log the result.
    ///
    /// The Morpho job (#11) and safe-mode detection (#12) are added
    /// as follow-on iterations.
    pub async fn tick(&self) {
        let now = Instant::now();
        let (iteration, snapshot) = {
            let mut w = self.state.write().await;
            let n = w.on_tick_start(now);
            let snap = w.clone();
            (n, snap)
        };

        tracing::info!(
            iteration,
            usdc_balance_usd = snapshot.usdc_balance_usd,
            safe_mode = snapshot.safe_mode,
            last_action = snapshot.last_action.as_deref().unwrap_or("(none)"),
            "tick"
        );

        // Yield strategy (#10). Safe-mode short-circuit lands in #12;
        // until then we always consult the strategy.
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
        let loop_ = AgentLoop::new(state.clone(), test_client(), test_config());
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
        let loop_ = AgentLoop::new(state.clone(), test_client(), test_config());
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
        let loop_ = AgentLoop::new(state.clone(), test_client(), test_config());
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
        let loop_ = AgentLoop::new(state.clone(), test_client(), test_config());
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
        let loop_ = AgentLoop::new(state, test_client(), test_config());
        let h1 = loop_.shutdown_handle();
        let h2 = h1.clone();
        // Both handles refer to the same notify; triggering either
        // wakes both waiters (Notify is reference-counted via Arc).
        h1.trigger();
        h2.trigger();
        // No panic = success
    }
}
