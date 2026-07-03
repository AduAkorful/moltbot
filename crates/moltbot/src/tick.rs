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
use crate::state::{AgentState, SharedState};
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
    /// The MCP client. Held for future iterations (#10 yield,
    /// #11 Morpho job, #12 safe mode) that will call KeeperHub from
    /// the tick. Currently unused by [`AgentLoop::tick`] — the
    /// session is initialized eagerly in `main` before the loop
    /// starts.
    #[allow(dead_code)]
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
    /// updates the timestamps, and logs the current USDC balance.
    ///
    /// For #9, this is intentionally a stub. Yield (#10), Morpho
    /// (#11), and safe-mode (#12) are added as follow-on iterations.
    pub async fn tick(&self) {
        let now = Instant::now();
        let iteration = {
            let mut w = self.state.write().await;
            w.on_tick_start(now)
        };

        let snapshot: AgentState = self.state.read().await.clone();

        tracing::info!(
            iteration,
            usdc_balance_usd = snapshot.usdc_balance_usd,
            safe_mode = snapshot.safe_mode,
            last_action = snapshot.last_action.as_deref().unwrap_or("(none)"),
            "tick"
        );

        // Future iterations wire in here:
        // - if safe_mode { return; }
        // - yield_strategy::decide_and_execute(&self.client, &mut *w).await;
        // - morpho_job::run(&self.client, &mut *w).await;
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
            w.set_usdc_balance(123.45);
        }
        let loop_ = AgentLoop::new(state.clone(), test_client(), test_config());
        loop_.tick().await;
        let s = state.read().await;
        assert_eq!(s.iteration, 1);
        assert!(s.started_at.is_some());
        assert!(s.last_tick_at.is_some());
        assert_eq!(s.usdc_balance_usd, 123.45);
    }

    #[tokio::test]
    async fn multiple_ticks_increment_iteration() {
        let state = new_shared_state();
        let loop_ = AgentLoop::new(state.clone(), test_client(), test_config());
        for _ in 0..3 {
            loop_.tick().await;
        }
        let s = state.read().await;
        assert_eq!(s.iteration, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn run_returns_on_ctrl_c() {
        // We can't easily inject a real SIGINT in a unit test, but we
        // can use the shutdown handle.
        let state = new_shared_state();
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
