//! The MoltBot dashboard.
//!
//! A small Axum server that renders the audit log (added in
//! #14) as a live-updating web page. The page polls
//! `/api/runs` and `/api/stats` every 10 seconds and shows
//! recent runs with their onchain tx hashes (linked to
//! Etherscan).
//!
//! # Routes
//!
//! | Method | Path                  | Purpose                              |
//! |--------|-----------------------|--------------------------------------|
//! | GET    | `/`                   | HTML dashboard (embedded `index.html`) |
//! | GET    | `/static/style.css`   | CSS (embedded `style.css`)           |
//! | GET    | `/static/app.js`      | JavaScript (embedded `app.js`)       |
//! | GET    | `/api/runs?limit=N`   | Recent runs as JSON                  |
//! | GET    | `/api/runs/:id`       | Single run with its actions, or 404  |
//! | GET    | `/api/stats`          | Aggregate stats (counts, kind table) |
//!
//! # Self-contained binary
//!
//! The HTML, CSS, and JS are embedded into the binary at
//! compile time via `include_str!`, so the dashboard has no
//! runtime file dependencies — the `moltbot` binary is
//! self-contained.
//!
//! # Layering
//!
//! The dashboard reads from the audit log directly; the agent
//! loop is the writer. There is no shared state beyond the
//! `Arc<AuditLog>` (which is `Clone`-cheap because the
//! underlying `SqlitePool` is an `Arc`).

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::audit::{ActionRow, AuditLog, RunWithActions};

// Embedded static assets. The `../static` paths are relative to
// `src/`, so the files live at the crate root under `static/`.
const INDEX_HTML: &str = include_str!("../static/index.html");
const STYLE_CSS: &str = include_str!("../static/style.css");
const APP_JS: &str = include_str!("../static/app.js");

/// Default listen address if the config doesn't set one. The
/// plan calls for `localhost:3030`.
pub const DEFAULT_DASHBOARD_ADDR: &str = "127.0.0.1:3030";

/// The dashboard's state. Cheap to clone (Arc-bump of the
/// underlying `SqlitePool`).
#[derive(Debug, Clone)]
pub struct DashboardState {
    pub audit: Arc<AuditLog>,
}

impl DashboardState {
    /// Build a new state from an `Arc<AuditLog>`.
    pub fn new(audit: Arc<AuditLog>) -> Self {
        Self { audit }
    }
}

/// Build the dashboard's Axum [`Router`]. The caller is
/// responsible for starting a server with this router (see
/// [`serve`]).
pub fn router(state: DashboardState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/runs", get(list_runs_handler))
        .route("/api/runs/:id", get(get_run_handler))
        .route("/api/stats", get(stats_handler))
        .route("/static/style.css", get(style_css_handler))
        .route("/static/app.js", get(app_js_handler))
        .with_state(state)
}

/// Convenience: start the dashboard server on `addr`. Blocks
/// the current task until the server exits. Returns an
/// `anyhow::Error` if the listener fails to bind.
pub async fn serve(state: DashboardState, addr: &str) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr, "dashboard listening");
    let router = router(state);
    axum::serve(listener, router).await?;
    Ok(())
}

// -- Handlers ---------------------------------------------------------------

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn style_css_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
}

async fn app_js_handler() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
        ],
        APP_JS,
    )
}

#[derive(Debug, Deserialize)]
struct ListRunsQuery {
    /// Maximum number of runs to return. Defaults to 20; clamped
    /// to `[1, 1000]`.
    #[serde(default)]
    limit: Option<i64>,
}

async fn list_runs_handler(
    State(state): State<DashboardState>,
    Query(q): Query<ListRunsQuery>,
) -> Result<Json<RunsResponse>, DashboardError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 1000);
    let mut runs: Vec<RunWithActions> = Vec::new();
    for run in state.audit.list_runs(limit).await? {
        let actions = state.audit.list_actions_for_run(run.id).await?;
        runs.push(RunWithActions { run, actions });
    }
    let resp = RunsResponse {
        runs: runs.into_iter().map(run_to_json).collect(),
    };
    Ok(Json(resp))
}

async fn get_run_handler(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> Result<Json<RunJson>, DashboardError> {
    let Some(rwa) = state.audit.get_run(id).await? else {
        return Err(DashboardError::NotFound);
    };
    Ok(Json(run_to_json(rwa)))
}

async fn stats_handler(
    State(state): State<DashboardState>,
) -> Result<Json<StatsJson>, DashboardError> {
    let s = state.audit.stats().await?;
    Ok(Json(StatsJson {
        total_runs: s.total_runs,
        total_actions: s.total_actions,
        total_x402_payments: s.total_x402_payments,
        actions_by_kind: s.actions_by_kind,
    }))
}

// -- JSON shapes ------------------------------------------------------------

#[derive(Debug, Serialize)]
struct RunsResponse {
    runs: Vec<RunJson>,
}

#[derive(Debug, Serialize)]
struct RunJson {
    id: i64,
    started_at: String,
    ended_at: Option<String>,
    iteration: i64,
    status: String,
    kind: String,
    actions: Vec<ActionJson>,
}

#[derive(Debug, Serialize)]
struct ActionJson {
    id: i64,
    kind: String,
    tx_hash: Option<String>,
    note: Option<String>,
    recorded_at: String,
}

#[derive(Debug, Serialize)]
struct StatsJson {
    total_runs: i64,
    total_actions: i64,
    total_x402_payments: i64,
    actions_by_kind: std::collections::BTreeMap<String, i64>,
}

fn run_to_json(rwa: RunWithActions) -> RunJson {
    RunJson {
        id: rwa.run.id,
        started_at: rwa.run.started_at,
        ended_at: rwa.run.ended_at,
        iteration: rwa.run.iteration,
        status: rwa.run.status,
        kind: rwa.run.kind,
        actions: rwa.actions.into_iter().map(action_to_json).collect(),
    }
}

fn action_to_json(a: ActionRow) -> ActionJson {
    ActionJson {
        id: a.id,
        kind: a.kind,
        tx_hash: a.tx_hash,
        note: a.note,
        recorded_at: a.recorded_at,
    }
}

// -- Error type -------------------------------------------------------------

/// Dashboard errors that map to HTTP responses.
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    /// The audit log returned an error.
    #[error("audit log error: {0}")]
    Audit(#[from] crate::audit::AuditError),
    /// The requested run does not exist.
    #[error("not found")]
    NotFound,
}

impl IntoResponse for DashboardError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
            Self::Audit(e) => {
                tracing::error!(error = %e, "dashboard audit error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::ActionRecord;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn test_app() -> (Router, Arc<AuditLog>) {
        let log = Arc::new(AuditLog::in_memory().await.unwrap());
        let state = DashboardState::new(Arc::clone(&log));
        (router(state), log)
    }

    /// Populate the audit log with a few runs + actions for
    /// the listing tests.
    async fn seed(log: &AuditLog) {
        let mut h1 = log.start_tick(1).await.unwrap();
        h1.record_action(
            &ActionRecord::new("yield::supply")
                .with_tx_hash("0xaaaa")
                .with_note("amount_usd=200.00"),
        )
        .await
        .unwrap();
        h1.commit().await.unwrap();

        let mut h2 = log.start_tick(2).await.unwrap();
        h2.record_action(&ActionRecord::new("morpho::check_hf"))
            .await
            .unwrap();
        h2.commit().await.unwrap();

        let mut h3 = log.start_tick(3).await.unwrap();
        h3.record_action(&ActionRecord::new("yield::supply"))
            .await
            .unwrap();
        h3.commit().await.unwrap();
    }

    #[tokio::test]
    async fn index_returns_html() {
        let (app, _log) = test_app().await;
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(ct.to_str().unwrap().starts_with("text/html"));
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("MoltBot"));
        assert!(s.contains("live audit"));
    }

    #[tokio::test]
    async fn style_css_returns_css() {
        let (app, _log) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/style.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(ct.to_str().unwrap().starts_with("text/css"));
    }

    #[tokio::test]
    async fn app_js_returns_javascript() {
        let (app, _log) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(ct.to_str().unwrap().starts_with("application/javascript"));
    }

    #[tokio::test]
    async fn api_runs_returns_seeded_runs() {
        let (app, log) = test_app().await;
        seed(&log).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let runs = body["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 3);
        // Newest first (run 3 first)
        assert_eq!(runs[0]["id"], 3);
        assert_eq!(runs[2]["id"], 1);
        assert_eq!(runs[0]["actions"][0]["kind"], "yield::supply");
        // Tx hash on the first run (run 1 has tx_hash 0xaaaa)
        let run1 = &runs[2];
        assert_eq!(run1["actions"][0]["tx_hash"], "0xaaaa");
        assert_eq!(run1["actions"][0]["note"], "amount_usd=200.00");
    }

    #[tokio::test]
    async fn api_runs_respects_limit() {
        let (app, log) = test_app().await;
        seed(&log).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs?limit=2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let runs = body["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0]["id"], 3);
        assert_eq!(runs[1]["id"], 2);
    }

    #[tokio::test]
    async fn api_runs_clamps_extreme_limits() {
        let (app, log) = test_app().await;
        seed(&log).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs?limit=99999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        // 3 seeded, less than 1000 clamp
        assert_eq!(body["runs"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn api_runs_by_id_returns_single() {
        let (app, log) = test_app().await;
        seed(&log).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["id"], 1);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["actions"][0]["kind"], "yield::supply");
    }

    #[tokio::test]
    async fn api_runs_by_id_returns_404_for_missing() {
        let (app, _log) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs/9999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_stats_returns_aggregates() {
        let (app, log) = test_app().await;
        seed(&log).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["total_runs"], 3);
        assert_eq!(body["total_actions"], 3);
        assert_eq!(body["total_x402_payments"], 0);
        let kinds = body["actions_by_kind"].as_object().unwrap();
        assert_eq!(kinds["yield::supply"], 2);
        assert_eq!(kinds["morpho::check_hf"], 1);
    }

    #[tokio::test]
    async fn api_stats_on_empty_db_returns_zeros() {
        let (app, _log) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["total_runs"], 0);
        assert_eq!(body["total_actions"], 0);
        assert_eq!(body["total_x402_payments"], 0);
        assert_eq!(body["actions_by_kind"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn default_dashboard_addr_is_localhost() {
        assert_eq!(DEFAULT_DASHBOARD_ADDR, "127.0.0.1:3030");
    }

    #[test]
    fn index_html_contains_polling_marker() {
        // Sanity-check: the embedded HTML references the 10s
        // polling cadence so a future refactor doesn't quietly
        // remove it.
        assert!(
            INDEX_HTML.contains("/api/runs") && INDEX_HTML.contains("MoltBot"),
            "embedded index.html is missing expected content"
        );
    }

    #[test]
    fn app_js_contains_poll_interval() {
        // Sanity-check: app.js polls every 10s.
        assert!(
            APP_JS.contains("POLL_MS = 10_000") || APP_JS.contains("10000"),
            "embedded app.js is missing the 10s poll interval"
        );
    }
}
