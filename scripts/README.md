# Scripts

One-off scripts for repo maintenance. Not part of the build.

## `create-issues.sh`

**Legacy — no longer used.** This script was used to bootstrap 31 GitHub
issues from `plans/next-steps.md`. We decided to work from the local doc
instead and removed the issues on Jul 3, 2026.

Kept here for reference in case the team wants to switch back to GitHub
Issues later, or fork the repo and use a different tracking approach.

**Requires (if re-run):** `GH_TOKEN` env var with `Issues: Read & Write`
permission on the `AduAkorful/moltbot` repo.

**Run:**
```sh
export GH_TOKEN="ghp_..."
./scripts/create-issues.sh
```
