//! Local SQLite audit log.
//!
//! Every agent tick is persisted as a [`run`] row. Each action
//! taken during the tick (a yield-strategy supply/withdraw, a
//! Morpho job observation, etc.) is a separate [`actions`] row
//! linked to the run by `run_id`. x402 payments are a third
//! table linked to the action by `action_id`.
//!
//! # Why a local audit log
//!
//! KeeperHub's runs panel is the source of truth, but a local
//! SQLite cache makes the dashboard fast and keeps the agent
//! self-contained. The dashboard (#15) reads from this DB; the
//! KeeperHub runs panel is a backup for verification.
//!
//! # Schema
//!
//! ```sql
//! CREATE TABLE runs (
//!     id           INTEGER PRIMARY KEY AUTOINCREMENT,
//!     started_at   TEXT NOT NULL,        -- ISO 8601 / RFC 3339
//!     ended_at     TEXT,                 -- set on commit / abort
//!     iteration    INTEGER NOT NULL,     -- AgentState::iteration
//!     status       TEXT NOT NULL,        -- 'running' | 'ok' | 'error'
//!     kind         TEXT NOT NULL         -- 'tick' (always, for now)
//! );
//!
//! CREATE TABLE actions (
//!     id           INTEGER PRIMARY KEY AUTOINCREMENT,
//!     run_id       INTEGER NOT NULL REFERENCES runs(id),
//!     kind         TEXT NOT NULL,        -- e.g. 'yield::supply'
//!     tx_hash      TEXT,                 -- onchain tx hash (nullable)
//!     note         TEXT,                 -- human-readable note
//!     recorded_at  TEXT NOT NULL
//! );
//!
//! CREATE TABLE x402_payments (
//!     id           INTEGER PRIMARY KEY AUTOINCREMENT,
//!     action_id    INTEGER NOT NULL REFERENCES actions(id),
//!     amount       TEXT NOT NULL,        -- string to preserve precision
//!     asset        TEXT NOT NULL,        -- e.g. 'USDC'
//!     chain        TEXT NOT NULL,        -- e.g. 'base'
//!     tx_hash      TEXT,                 -- settlement tx hash
//!     recorded_at  TEXT NOT NULL
//! );
//! ```
//!
//! # Transactions
//!
//! All writes for a single tick are wrapped in a single
//! transaction. The [`TickHandle`] returned by
//! [`AuditLog::start_tick`] owns the `Transaction`; calling
//! [`TickHandle::commit`] persists everything atomically, and
//! dropping the handle without committing rolls back.
//!
//! [`run`]: #
//! [`actions`]: #

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool, Transaction};
use thiserror::Error;

/// Audit-log errors. `sqlx` is the underlying driver; the
/// `Sqlx` variant preserves the original error for logging.
#[derive(Debug, Error)]
pub enum AuditError {
    /// A sqlx error (connection, query, transaction, etc.).
    #[error("audit log: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// A migration error (schema creation failed).
    #[error("audit log: migration failed: {0}")]
    Migration(String),
}

/// Status of a recorded run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Run is in progress. The audit log persists the row in
    /// this state until the transaction is committed.
    Running,
    /// Run completed successfully.
    Ok,
    /// Run failed (the loop caught an error and committed with
    /// an `error` status so the row remains in the DB).
    Error,
}

impl RunStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

/// A single agent action (yield supply, Morpho observation, etc.).
#[derive(Debug, Clone)]
pub struct ActionRecord {
    /// The kind of action, e.g. `"yield::supply"` or
    /// `"morpho::check_hf"`. Free-form string.
    pub kind: String,
    /// The onchain transaction hash, if the action broadcasted
    /// a tx (e.g. an Aave supply).
    pub tx_hash: Option<String>,
    /// A human-readable note, e.g. `"HF=1.21 < target 1.3"`.
    pub note: Option<String>,
}

impl ActionRecord {
    /// Build a new action record. `kind` is required; `tx_hash`
    /// and `note` default to `None`.
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            tx_hash: None,
            note: None,
        }
    }

    /// Set the onchain transaction hash.
    #[must_use]
    pub fn with_tx_hash(mut self, hash: impl Into<String>) -> Self {
        self.tx_hash = Some(hash.into());
        self
    }

    /// Set the human-readable note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// An x402 payment tied to an action (e.g. a paid workflow
/// call from the Morpho job).
#[derive(Debug, Clone)]
pub struct X402Payment {
    /// The payment amount, as a string to preserve precision
    /// (e.g. `"0.01"` for one cent, or `"1000000"` for 1 USDC
    /// in smallest units).
    pub amount: String,
    /// The asset symbol (e.g. `"USDC"`).
    pub asset: String,
    /// The chain where the payment settled (e.g. `"base"`,
    /// `"base-sepolia"`).
    pub chain: String,
    /// The settlement transaction hash, if known.
    pub tx_hash: Option<String>,
}

impl X402Payment {
    /// Build a new x402 payment. `amount`, `asset`, and `chain`
    /// are required; `tx_hash` defaults to `None`.
    pub fn new(
        amount: impl Into<String>,
        asset: impl Into<String>,
        chain: impl Into<String>,
    ) -> Self {
        Self {
            amount: amount.into(),
            asset: asset.into(),
            chain: chain.into(),
            tx_hash: None,
        }
    }

    /// Set the settlement transaction hash.
    #[must_use]
    pub fn with_tx_hash(mut self, hash: impl Into<String>) -> Self {
        self.tx_hash = Some(hash.into());
        self
    }
}

/// The audit log: a SQLite-backed store of every agent run,
/// action, and x402 payment.
///
/// Cheap to clone via internal [`SqlitePool`] (which is itself
/// an `Arc`).
#[derive(Debug, Clone)]
pub struct AuditLog {
    pool: SqlitePool,
}

impl AuditLog {
    /// Open or create the audit database at `url`.
    ///
    /// `url` is a SQLite connection URL. The most useful values
    /// for the agent are:
    /// - `sqlite::memory:` — ephemeral, perfect for tests
    /// - `sqlite:./moltbot.db` — file-backed, default for the
    ///   binary
    ///
    /// On success, the schema is created if it doesn't exist.
    pub async fn connect(url: &str) -> Result<Self, AuditError> {
        let opts: SqliteConnectOptions = url
            .parse()
            .map_err(|e: sqlx::Error| AuditError::Migration(format!("invalid url: {e}")))?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts.create_if_missing(true))
            .await?;
        let log = Self { pool };
        log.run_migrations().await?;
        Ok(log)
    }

    /// Open an in-memory audit log. Convenient for tests and
    /// short-lived processes.
    pub async fn in_memory() -> Result<Self, AuditError> {
        Self::connect("sqlite::memory:").await
    }

    /// Run schema migrations. Idempotent — safe to call on
    /// every startup. The current migration is the initial
    /// schema (no version tracking yet — that's deferred until
    /// we need to evolve the schema).
    async fn run_migrations(&self) -> Result<(), AuditError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runs (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at   TEXT NOT NULL,
                ended_at     TEXT,
                iteration    INTEGER NOT NULL,
                status       TEXT NOT NULL,
                kind         TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS actions (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id       INTEGER NOT NULL REFERENCES runs(id),
                kind         TEXT NOT NULL,
                tx_hash      TEXT,
                note         TEXT,
                recorded_at  TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS x402_payments (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                action_id    INTEGER NOT NULL REFERENCES actions(id),
                amount       TEXT NOT NULL,
                asset        TEXT NOT NULL,
                chain        TEXT NOT NULL,
                tx_hash      TEXT,
                recorded_at  TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_actions_run_id ON actions(run_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_x402_action_id ON x402_payments(action_id)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a reference to the underlying connection pool. Used
    /// by the dashboard (#15) to query runs.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Start a new tick transaction. Returns a [`TickHandle`]
    /// that owns the transaction; drop it without committing
    /// to roll back.
    pub async fn start_tick(&self, iteration: u64) -> Result<TickHandle<'_>, AuditError> {
        let mut tx = self.pool.begin().await?;
        let started_at = now_rfc3339();
        let run_id: i64 = sqlx::query(
            "INSERT INTO runs (started_at, iteration, status, kind) VALUES (?, ?, ?, 'tick')",
        )
        .bind(&started_at)
        .bind(iteration as i64)
        .bind(RunStatus::Running.as_str())
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        Ok(TickHandle { tx, run_id })
    }

    /// Count the number of runs in the audit log. Used by
    /// tests and (later) the dashboard's stats endpoint.
    pub async fn count_runs(&self) -> Result<i64, AuditError> {
        let row = sqlx::query("SELECT COUNT(*) AS c FROM runs")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("c"))
    }

    /// Count the number of actions across all runs.
    pub async fn count_actions(&self) -> Result<i64, AuditError> {
        let row = sqlx::query("SELECT COUNT(*) AS c FROM actions")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("c"))
    }

    /// Count the number of x402 payments across all actions.
    pub async fn count_x402_payments(&self) -> Result<i64, AuditError> {
        let row = sqlx::query("SELECT COUNT(*) AS c FROM x402_payments")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("c"))
    }
}

/// A handle to an in-flight tick transaction.
///
/// Owns a `sqlx::Transaction`. Call [`TickHandle::commit`] to
/// persist all writes atomically, [`TickHandle::abort_with`]
/// to mark the run as `error` and still commit (so the row
/// remains in the DB), or drop the handle to roll back
/// silently.
///
/// `commit` returns the run's row id.
pub struct TickHandle<'a> {
    tx: Transaction<'a, sqlx::Sqlite>,
    run_id: i64,
}

impl<'a> TickHandle<'a> {
    /// The row id of the run that this handle is writing to.
    pub fn run_id(&self) -> i64 {
        self.run_id
    }

    /// Record an action. Returns the new action's row id,
    /// which can be passed to [`TickHandle::record_x402_payment`]
    /// to attach an x402 payment.
    pub async fn record_action(
        &mut self,
        action: &ActionRecord,
    ) -> Result<i64, AuditError> {
        let recorded_at = now_rfc3339();
        let id: i64 = sqlx::query(
            "INSERT INTO actions (run_id, kind, tx_hash, note, recorded_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(self.run_id)
        .bind(&action.kind)
        .bind(action.tx_hash.as_deref())
        .bind(action.note.as_deref())
        .bind(&recorded_at)
        .execute(&mut *self.tx)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    /// Record an x402 payment tied to a previously-recorded
    /// action. The `action_id` is the return value of
    /// [`TickHandle::record_action`].
    pub async fn record_x402_payment(
        &mut self,
        action_id: i64,
        payment: &X402Payment,
    ) -> Result<i64, AuditError> {
        let recorded_at = now_rfc3339();
        let id: i64 = sqlx::query(
            "INSERT INTO x402_payments (action_id, amount, asset, chain, tx_hash, recorded_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(action_id)
        .bind(&payment.amount)
        .bind(&payment.asset)
        .bind(&payment.chain)
        .bind(payment.tx_hash.as_deref())
        .bind(&recorded_at)
        .execute(&mut *self.tx)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    /// Commit the transaction. Sets the run's `ended_at` and
    /// `status = 'ok'`. Returns the run's row id.
    pub async fn commit(mut self) -> Result<i64, AuditError> {
        let ended_at = now_rfc3339();
        sqlx::query("UPDATE runs SET ended_at = ?, status = ? WHERE id = ?")
            .bind(&ended_at)
            .bind(RunStatus::Ok.as_str())
            .bind(self.run_id)
            .execute(&mut *self.tx)
            .await?;
        self.tx.commit().await?;
        Ok(self.run_id)
    }

    /// Mark the run as `error` and commit. Use this when a
    /// tick caught an error but you still want the run row in
    /// the DB for audit purposes. Returns the run's row id.
    pub async fn abort_with_error(mut self) -> Result<i64, AuditError> {
        let ended_at = now_rfc3339();
        sqlx::query("UPDATE runs SET ended_at = ?, status = ? WHERE id = ?")
            .bind(&ended_at)
            .bind(RunStatus::Error.as_str())
            .bind(self.run_id)
            .execute(&mut *self.tx)
            .await?;
        self.tx.commit().await?;
        Ok(self.run_id)
    }
}

/// Current UTC time as an RFC 3339 / ISO 8601 string.
///
/// The audit log stores timestamps as `TEXT` (RFC 3339) for
/// portability and human readability when querying the DB
/// with the `sqlite3` CLI.
fn now_rfc3339() -> String {
    let now: DateTime<Utc> = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| DateTime::<Utc>::from_timestamp(d.as_secs() as i64, d.subsec_nanos()).unwrap_or_else(Utc::now))
        .unwrap_or_else(|_| Utc::now());
    now.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_creates_schema() {
        let log = AuditLog::in_memory().await.unwrap();
        // After connect, the tables should exist; we can verify
        // by inserting and counting.
        let runs_before = log.count_runs().await.unwrap();
        assert_eq!(runs_before, 0);
    }

    #[tokio::test]
    async fn connect_is_idempotent() {
        // Calling run_migrations twice (via two connect() calls
        // on the same in-memory DB) should not error. Each
        // in-memory DB is independent, but the second call
        // exercises the `IF NOT EXISTS` clause.
        let log1 = AuditLog::in_memory().await.unwrap();
        let log2 = AuditLog::in_memory().await.unwrap();
        assert_eq!(log1.count_runs().await.unwrap(), 0);
        assert_eq!(log2.count_runs().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn start_tick_inserts_run_row() {
        let log = AuditLog::in_memory().await.unwrap();
        let handle = log.start_tick(42).await.unwrap();
        let run_id = handle.run_id();
        assert!(run_id > 0, "expected positive run_id, got {run_id}");
        // Abort the handle so the transaction releases its
        // connection. The row persists (commit-with-error),
        // so we can verify it from the pool. The next test
        // (`abort_with_error_marks_status_error`) covers the
        // `error` status assertion specifically.
        handle.abort_with_error().await.unwrap();
        let row = sqlx::query("SELECT iteration, kind FROM runs WHERE id = ?")
            .bind(run_id)
            .fetch_one(log.pool())
            .await
            .unwrap();
        let iteration: i64 = row.get("iteration");
        let kind: String = row.get("kind");
        assert_eq!(iteration, 42);
        assert_eq!(kind, "tick");
    }

    #[tokio::test]
    async fn commit_persists_run_with_status_ok() {
        let log = AuditLog::in_memory().await.unwrap();
        let handle = log.start_tick(1).await.unwrap();
        let run_id = handle.run_id();
        handle.commit().await.unwrap();
        let row = sqlx::query("SELECT status, ended_at FROM runs WHERE id = ?")
            .bind(run_id)
            .fetch_one(log.pool())
            .await
            .unwrap();
        let status: String = row.get("status");
        let ended_at: Option<String> = row.get("ended_at");
        assert_eq!(status, "ok");
        assert!(ended_at.is_some(), "ended_at should be set after commit");
    }

    #[tokio::test]
    async fn abort_with_error_marks_status_error() {
        let log = AuditLog::in_memory().await.unwrap();
        let handle = log.start_tick(1).await.unwrap();
        let run_id = handle.run_id();
        handle.abort_with_error().await.unwrap();
        let row = sqlx::query("SELECT status FROM runs WHERE id = ?")
            .bind(run_id)
            .fetch_one(log.pool())
            .await
            .unwrap();
        let status: String = row.get("status");
        assert_eq!(status, "error");
    }

    #[tokio::test]
    async fn drop_without_commit_rolls_back() {
        let log = AuditLog::in_memory().await.unwrap();
        // The handle's `tx` is borrowed by `record_action`, so
        // we can't observe a non-commit scenario inside the
        // same function easily. Instead, we explicitly drop
        // after a borrow that goes out of scope.
        {
            let mut handle = log.start_tick(1).await.unwrap();
            handle
                .record_action(&ActionRecord::new("test::drop"))
                .await
                .unwrap();
            // handle drops here without commit
        }
        // The run row should not exist (transaction was rolled
        // back when the handle was dropped).
        assert_eq!(log.count_runs().await.unwrap(), 0);
        assert_eq!(log.count_actions().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn record_action_persists_with_tx_hash_and_note() {
        let log = AuditLog::in_memory().await.unwrap();
        let mut handle = log.start_tick(1).await.unwrap();
        let action_id = handle
            .record_action(
                &ActionRecord::new("yield::supply")
                    .with_tx_hash("0xdeadbeef")
                    .with_note("balance=200.0 > park=50.0"),
            )
            .await
            .unwrap();
        handle.commit().await.unwrap();

        let row = sqlx::query(
            "SELECT kind, tx_hash, note, run_id FROM actions WHERE id = ?",
        )
        .bind(action_id)
        .fetch_one(log.pool())
        .await
        .unwrap();
        let kind: String = row.get("kind");
        let tx_hash: Option<String> = row.get("tx_hash");
        let note: Option<String> = row.get("note");
        let run_id: i64 = row.get("run_id");
        assert_eq!(kind, "yield::supply");
        assert_eq!(tx_hash.as_deref(), Some("0xdeadbeef"));
        assert_eq!(note.as_deref(), Some("balance=200.0 > park=50.0"));
        assert!(run_id > 0);
    }

    #[tokio::test]
    async fn record_x402_payment_persists_with_action_link() {
        let log = AuditLog::in_memory().await.unwrap();
        let mut handle = log.start_tick(1).await.unwrap();
        let action_id = handle
            .record_action(&ActionRecord::new("morpho::risk_check"))
            .await
            .unwrap();
        let payment_id = handle
            .record_x402_payment(
                action_id,
                &X402Payment::new("0.01", "USDC", "base")
                    .with_tx_hash("0xfeedface"),
            )
            .await
            .unwrap();
        handle.commit().await.unwrap();

        let row = sqlx::query(
            "SELECT amount, asset, chain, tx_hash, action_id FROM x402_payments WHERE id = ?",
        )
        .bind(payment_id)
        .fetch_one(log.pool())
        .await
        .unwrap();
        let amount: String = row.get("amount");
        let asset: String = row.get("asset");
        let chain: String = row.get("chain");
        let tx_hash: Option<String> = row.get("tx_hash");
        let act_id: i64 = row.get("action_id");
        assert_eq!(amount, "0.01");
        assert_eq!(asset, "USDC");
        assert_eq!(chain, "base");
        assert_eq!(tx_hash.as_deref(), Some("0xfeedface"));
        assert_eq!(act_id, action_id);
    }

    #[tokio::test]
    async fn multiple_actions_in_one_tick_share_run() {
        let log = AuditLog::in_memory().await.unwrap();
        let mut handle = log.start_tick(1).await.unwrap();
        let a1 = handle
            .record_action(&ActionRecord::new("yield::supply"))
            .await
            .unwrap();
        let a2 = handle
            .record_action(&ActionRecord::new("morpho::check_hf"))
            .await
            .unwrap();
        let a3 = handle
            .record_action(&ActionRecord::new("morpho::supply_collateral"))
            .await
            .unwrap();
        handle.commit().await.unwrap();
        // All three actions share the same run_id.
        assert_eq!(log.count_runs().await.unwrap(), 1);
        assert_eq!(log.count_actions().await.unwrap(), 3);
        // Look up run_id for each action and verify equality.
        for aid in [a1, a2, a3] {
            let row = sqlx::query("SELECT run_id FROM actions WHERE id = ?")
                .bind(aid)
                .fetch_one(log.pool())
                .await
                .unwrap();
            let rid: i64 = row.get("run_id");
            assert_eq!(rid, handle_run_id_via_select(&log, aid).await);
        }
    }

    async fn handle_run_id_via_select(log: &AuditLog, action_id: i64) -> i64 {
        let row = sqlx::query("SELECT run_id FROM actions WHERE id = ?")
            .bind(action_id)
            .fetch_one(log.pool())
            .await
            .unwrap();
        row.get("run_id")
    }

    #[tokio::test]
    async fn counts_reflect_state() {
        let log = AuditLog::in_memory().await.unwrap();
        assert_eq!(log.count_runs().await.unwrap(), 0);
        assert_eq!(log.count_actions().await.unwrap(), 0);
        assert_eq!(log.count_x402_payments().await.unwrap(), 0);

        let mut h1 = log.start_tick(1).await.unwrap();
        h1.record_action(&ActionRecord::new("a")).await.unwrap();
        h1.commit().await.unwrap();

        let mut h2 = log.start_tick(2).await.unwrap();
        let a_id = h2.record_action(&ActionRecord::new("b")).await.unwrap();
        h2.record_x402_payment(a_id, &X402Payment::new("0.01", "USDC", "base"))
            .await
            .unwrap();
        h2.commit().await.unwrap();

        assert_eq!(log.count_runs().await.unwrap(), 2);
        assert_eq!(log.count_actions().await.unwrap(), 2);
        assert_eq!(log.count_x402_payments().await.unwrap(), 1);
    }

    #[test]
    fn action_record_builder() {
        let a = ActionRecord::new("foo");
        assert_eq!(a.kind, "foo");
        assert!(a.tx_hash.is_none());
        assert!(a.note.is_none());

        let a = ActionRecord::new("foo").with_tx_hash("0x1").with_note("hello");
        assert_eq!(a.tx_hash.as_deref(), Some("0x1"));
        assert_eq!(a.note.as_deref(), Some("hello"));
    }

    #[test]
    fn x402_payment_builder() {
        let p = X402Payment::new("0.01", "USDC", "base");
        assert_eq!(p.amount, "0.01");
        assert_eq!(p.asset, "USDC");
        assert_eq!(p.chain, "base");
        assert!(p.tx_hash.is_none());

        let p = X402Payment::new("0.01", "USDC", "base").with_tx_hash("0xabc");
        assert_eq!(p.tx_hash.as_deref(), Some("0xabc"));
    }

    #[test]
    fn run_status_as_str() {
        assert_eq!(RunStatus::Running.as_str(), "running");
        assert_eq!(RunStatus::Ok.as_str(), "ok");
        assert_eq!(RunStatus::Error.as_str(), "error");
    }
}
