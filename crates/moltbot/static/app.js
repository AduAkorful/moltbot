// MoltBot dashboard — polls /api/runs and /api/stats every 10s.
// Vanilla JS, no framework. Each section re-renders on update.

const POLL_MS = 10_000;

const $ = (id) => document.getElementById(id);

const etherscanTxUrl = (hash) =>
  `https://etherscan.io/tx/${hash}`;

const formatTime = (iso) => {
  if (!iso) return "—";
  // Render as HH:MM:SS in local time (we don't need full dates in the table).
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString();
};

const renderStats = (stats) => {
  $("stat-runs").textContent = stats.total_runs;
  $("stat-actions").textContent = stats.total_actions;
  $("stat-payments").textContent = stats.total_x402_payments;
  $("stat-updated").textContent = new Date().toLocaleTimeString();

  const kinds = Object.entries(stats.actions_by_kind || {});
  const tbody = $("kind-tbody");
  if (kinds.length === 0) {
    tbody.innerHTML = `<tr><td colspan="2" class="dim">no actions yet</td></tr>`;
    return;
  }
  // Sort by count desc
  kinds.sort((a, b) => b[1] - a[1]);
  tbody.innerHTML = kinds
    .map(([kind, count]) =>
      `<tr><td class="mono">${kind}</td><td>${count}</td></tr>`)
    .join("");
};

const renderRuns = (runs) => {
  const tbody = $("runs-tbody");
  if (!runs || runs.length === 0) {
    tbody.innerHTML = `<tr><td colspan="6" class="dim">no runs yet</td></tr>`;
    return;
  }
  tbody.innerHTML = runs
    .map((r) => {
      const pillClass = r.status === "ok" ? "ok"
                      : r.status === "error" ? "err"
                      : "run";
      const actions = r.actions || [];
      const txHashes = actions
        .filter((a) => a.tx_hash)
        .map((a) => `<a href="${etherscanTxUrl(a.tx_hash)}" target="_blank" rel="noopener">${a.tx_hash.slice(0, 10)}…</a>`)
        .join(", ") || "<span class='dim'>—</span>";
      return `
        <tr>
          <td class="mono">${r.id}</td>
          <td>${formatTime(r.started_at)}</td>
          <td>${r.iteration}</td>
          <td><span class="pill ${pillClass}">${r.status}</span></td>
          <td>${actions.length === 0 ? "<span class='dim'>0</span>" : actions.map((a) => `<span class="mono">${a.kind}</span>`).join(", ")}</td>
          <td>${txHashes}</td>
        </tr>`;
    })
    .join("");
};

const fetchJson = async (url) => {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${url}: ${res.status}`);
  return res.json();
};

const poll = async () => {
  try {
    const [runsResp, stats] = await Promise.all([
      fetchJson("/api/runs?limit=20"),
      fetchJson("/api/stats"),
    ]);
    renderRuns(runsResp.runs);
    renderStats(stats);
  } catch (e) {
    console.error("poll failed", e);
  }
};

document.addEventListener("DOMContentLoaded", () => {
  poll();
  setInterval(poll, POLL_MS);
});
