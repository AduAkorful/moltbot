//! The job system.
//!
//! A [`Job`] is a unit of work that runs on every agent tick. Jobs
//! are independent of each other and of the yield strategy — the
//! dispatcher just iterates over them, gates each on its
//! [`Job::should_run`] predicate, and runs the ones that pass.
//!
//! # Why a trait
//!
//! The plan's acceptance criterion for iteration #11 is that
//! "adding a second job (e.g., `price_alert`) requires <50 lines."
//! A trait + `Vec<Box<dyn Job>>` registry is the smallest design
//! that meets that bar: a new job is one struct + one `impl Job`
//! block + one `JobRegistry::with(job)` call. No enum variants or
//! match arms to update.
//!
//! # Object safety
//!
//! Native `async fn` in a trait is **not** object-safe. To keep the
//! registry dynamic, the trait exposes a `tick` method that returns
//! a boxed [`std::future::Future`]. The future is tied to the
//! borrow of [`JobContext`] by lifetime `'a`, so no `'static`
//! constraint is needed.
//!
//! [`JobContext`]: JobContext

use std::future::Future;
use std::pin::Pin;

use keeperhub_rs::mcp::McpClient;
use keeperhub_rs::Result;

use crate::config::AgentConfig;
use crate::state::AgentState;

/// A boxed future tied to the lifetime of the [`JobContext`] borrow.
/// Used as the return type of [`Job::tick`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a job's tick returns. The agent loop records
/// [`JobOutcome::action_taken`] (if any) as the loop's
/// `last_action` and logs the rest.
///
/// `tx_hash` is set by write jobs that successfully broadcast an
/// onchain transaction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobOutcome {
    /// A short, human-readable label of the action the job took
    /// (e.g. `"morpho::supply_collateral"`). `None` when the job
    /// observed but did not act.
    pub action_taken: Option<String>,
    /// The transaction hash of any onchain action. `None` for
    /// read-only ticks.
    pub tx_hash: Option<String>,
    /// A free-form note for the audit log (e.g. `"HF=1.21 < target 1.3"`).
    /// Not currently persisted — present for #14 (SQLite audit log).
    pub note: Option<String>,
}

impl JobOutcome {
    /// A no-op outcome: the job ran but had nothing to do.
    pub fn noop() -> Self {
        Self::default()
    }

    /// An outcome with just a note, no action.
    pub fn observed(note: impl Into<String>) -> Self {
        Self {
            note: Some(note.into()),
            ..Self::default()
        }
    }

    /// An outcome recording an action and (optionally) a tx hash.
    pub fn acted(
        action: impl Into<String>,
        tx_hash: Option<String>,
        note: Option<String>,
    ) -> Self {
        Self {
            action_taken: Some(action.into()),
            tx_hash,
            note,
        }
    }
}

/// The context passed to a job's [`Job::tick`].
///
/// Borrows from the [`AgentLoop`]'s fields. The job can read the
/// current state and config and call KeeperHub through the MCP
/// client, but cannot mutate the state — the loop owns the write
/// lock and reads the outcome's `action_taken` to update
/// `state.last_action` itself.
pub struct JobContext<'a> {
    pub client: &'a McpClient,
    pub config: &'a AgentConfig,
    pub state: &'a AgentState,
}

/// A unit of work that runs on every agent tick.
///
/// The trait is object-safe: [`Job::tick`] returns a [`BoxFuture`]
/// rather than using native `async fn` (which is not
/// object-safe). The future is tied to the [`JobContext`] borrow.
pub trait Job: Send + Sync {
    /// A stable identifier used in tracing logs (e.g. `"morpho_health"`).
    fn name(&self) -> &'static str;

    /// Gate the job for the current tick. Returning `false` skips
    /// the job without an error. Common reasons to return `false`:
    /// missing config (e.g. no `morpho_market_id` set), the agent
    /// is in safe mode, or the job's source data is offline.
    fn should_run(&self, state: &AgentState, config: &AgentConfig) -> bool;

    /// Run one tick of the job. The returned future borrows
    /// `ctx`; the registry awaits it inline.
    ///
    /// Errors from network/protocol calls should be returned
    /// (not panicked on); the registry logs them and continues
    /// to the next job.
    fn tick<'a>(&'a self, ctx: &'a JobContext<'a>) -> BoxFuture<'a, Result<JobOutcome>>;
}

/// An ordered collection of jobs that the agent runs on every tick.
///
/// Construction:
/// ```ignore
/// let registry = JobRegistry::new()
///     .with(MorphoHealthJob::new());
/// ```
///
/// `with` is a builder method that returns `self` by value, so
/// calls can be chained. A new `JobRegistry` is empty (a no-op).
#[derive(Default)]
pub struct JobRegistry {
    jobs: Vec<Box<dyn Job>>,
}

impl std::fmt::Debug for JobRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobRegistry")
            .field("count", &self.jobs.len())
            .field(
                "names",
                &self.jobs.iter().map(|j| j.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl JobRegistry {
    /// An empty registry. `run_all` on an empty registry is a
    /// no-op.
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Append a job. Returns `self` for chaining.
    pub fn with<J: Job + 'static>(mut self, job: J) -> Self {
        self.jobs.push(Box::new(job));
        self
    }

    /// The number of jobs registered. Used in tests and logs.
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// True if no jobs are registered.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Run every enabled job once. Errors are logged with the
    /// job's `name()` and absorbed; a failing job does not stop
    /// the rest. Returns a [`JobRunReport`] listing each
    /// successful outcome and a count of errors so the caller
    /// can decide whether to mark the run as failed.
    pub async fn run_all(&self, ctx: &JobContext<'_>) -> JobRunReport {
        let mut outcomes = Vec::new();
        let mut error_count: usize = 0;
        for job in &self.jobs {
            if !job.should_run(ctx.state, ctx.config) {
                tracing::debug!(job = job.name(), "job: should_run=false, skipping");
                continue;
            }
            match job.tick(ctx).await {
                Ok(outcome) => {
                    if outcome.action_taken.is_some() || outcome.note.is_some() {
                        tracing::info!(
                            job = job.name(),
                            action = outcome.action_taken.as_deref().unwrap_or("(none)"),
                            tx_hash = outcome.tx_hash.as_deref().unwrap_or("(none)"),
                            note = outcome.note.as_deref().unwrap_or("(none)"),
                            "job: tick ok"
                        );
                    } else {
                        tracing::debug!(job = job.name(), "job: tick ok (noop)");
                    }
                    outcomes.push(outcome);
                }
                Err(e) => {
                    error_count += 1;
                    tracing::error!(
                        job = job.name(),
                        error = %e,
                        "job: tick failed"
                    );
                }
            }
        }
        JobRunReport {
            outcomes,
            error_count,
        }
    }
}

/// The result of a single [`JobRegistry::run_all`] invocation.
///
/// `outcomes` lists every job that ran (successfully). Disabled
/// jobs (`should_run == false`) are silently skipped and do
/// not appear here. `error_count` is the number of jobs that
/// returned an error during their tick; those errors are
/// logged by [`JobRegistry::run_all`] and not surfaced via
/// the outcomes (use `error_count > 0` to mark the run as
/// failed).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct JobRunReport {
    /// Successful job outcomes, in registry order.
    pub outcomes: Vec<JobOutcome>,
    /// Number of jobs that errored during this tick.
    pub error_count: usize,
}

impl JobRunReport {
    /// Did any job error during this tick?
    pub fn had_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Number of jobs that produced an `action_taken`.
    pub fn acted_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.action_taken.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// A test job that always runs, increments a counter, and
    /// returns a no-op outcome.
    struct CounterJob {
        counter: Arc<AtomicU32>,
    }

    impl Job for CounterJob {
        fn name(&self) -> &'static str {
            "counter"
        }
        fn should_run(&self, _state: &AgentState, _config: &AgentConfig) -> bool {
            true
        }
        fn tick<'a>(&'a self, _ctx: &'a JobContext<'a>) -> BoxFuture<'a, Result<JobOutcome>> {
            Box::pin(async {
                self.counter.fetch_add(1, Ordering::SeqCst);
                Ok(JobOutcome::noop())
            })
        }
    }

    /// A test job that only runs when its `enabled` flag is true.
    struct GatedJob {
        enabled: bool,
        counter: Arc<AtomicU32>,
    }

    impl Job for GatedJob {
        fn name(&self) -> &'static str {
            "gated"
        }
        fn should_run(&self, _state: &AgentState, _config: &AgentConfig) -> bool {
            self.enabled
        }
        fn tick<'a>(&'a self, _ctx: &'a JobContext<'a>) -> BoxFuture<'a, Result<JobOutcome>> {
            Box::pin(async {
                self.counter.fetch_add(1, Ordering::SeqCst);
                Ok(JobOutcome::noop())
            })
        }
    }

    /// A test job that always errors.
    struct FailingJob;

    impl Job for FailingJob {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn should_run(&self, _state: &AgentState, _config: &AgentConfig) -> bool {
            true
        }
        fn tick<'a>(&'a self, _ctx: &'a JobContext<'a>) -> BoxFuture<'a, Result<JobOutcome>> {
            Box::pin(async {
                Err(keeperhub_rs::Error::Config(
                    "synthetic test error".to_string(),
                ))
            })
        }
    }

    fn test_config() -> AgentConfig {
        AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            ..AgentConfig::default()
        }
    }

    fn test_state() -> AgentState {
        AgentState::new()
    }

    fn test_client() -> Arc<McpClient> {
        Arc::new(McpClient::new("https://app.keeperhub.com/mcp", "kh_test"))
    }

    #[test]
    fn empty_registry_is_a_noop() {
        let r = JobRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[tokio::test]
    async fn registry_runs_every_job() {
        let counter = Arc::new(AtomicU32::new(0));
        let r = JobRegistry::new()
            .with(CounterJob {
                counter: Arc::clone(&counter),
            })
            .with(CounterJob {
                counter: Arc::clone(&counter),
            });
        let ctx = JobContext {
            client: &test_client(),
            config: &test_config(),
            state: &test_state(),
        };
        let report = r.run_all(&ctx).await;
        assert_eq!(report.outcomes.len(), 2);
        assert_eq!(report.error_count, 0);
        assert!(!report.had_errors());
        assert_eq!(report.acted_count(), 0);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn registry_skips_disabled_jobs() {
        let counter = Arc::new(AtomicU32::new(0));
        let r = JobRegistry::new()
            .with(GatedJob {
                enabled: true,
                counter: Arc::clone(&counter),
            })
            .with(GatedJob {
                enabled: false,
                counter: Arc::clone(&counter),
            });
        let ctx = JobContext {
            client: &test_client(),
            config: &test_config(),
            state: &test_state(),
        };
        let report = r.run_all(&ctx).await;
        assert_eq!(report.outcomes.len(), 1);
        assert_eq!(report.error_count, 0);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn registry_continues_past_failing_job() {
        let counter = Arc::new(AtomicU32::new(0));
        let r = JobRegistry::new()
            .with(FailingJob)
            .with(CounterJob {
                counter: Arc::clone(&counter),
            });
        let ctx = JobContext {
            client: &test_client(),
            config: &test_config(),
            state: &test_state(),
        };
        let report = r.run_all(&ctx).await;
        // The failing job returns Err (absorbed by the registry);
        // the counter job still runs and returns an outcome.
        assert_eq!(report.outcomes.len(), 1);
        assert_eq!(report.error_count, 1);
        assert!(report.had_errors());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn job_outcome_constructors() {
        assert_eq!(JobOutcome::noop(), JobOutcome::default());
        let o = JobOutcome::observed("HF=1.21");
        assert_eq!(o.action_taken, None);
        assert_eq!(o.note.as_deref(), Some("HF=1.21"));
        let o = JobOutcome::acted("morpho::supply_collateral", Some("0xabc".into()), None);
        assert_eq!(o.action_taken.as_deref(), Some("morpho::supply_collateral"));
        assert_eq!(o.tx_hash.as_deref(), Some("0xabc"));
    }
}
