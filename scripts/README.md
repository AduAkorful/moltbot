# Scripts

One-off scripts for repo maintenance. Not part of the build.

## `create-issues.sh`

Creates all 31 GitHub issues from `plans/next-steps.md`. Idempotent-ish (issues
will be created again if run twice — guard by checking existing issues first
if you need true idempotency).

**Requires:** `GH_TOKEN` env var with `Issues: Read & Write` permission on the
`AduAkorful/moltbot` repo.

**Run:**
```sh
export GH_TOKEN="ghp_..."
./scripts/create-issues.sh
```

This was used to bootstrap the issue tracker. After the initial creation, work
on issues directly through the GitHub UI.
