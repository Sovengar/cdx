# cdx-rs — Agent Instructions

## Build & Deploy

After making source changes, you MUST build the release binary and copy it to `~/.local/bin/` so the user can test it.

The `cdx` shell command calls `~/.local/bin/cdx`. Source changes alone won't be visible until the binary is deployed there.

```bash
cargo build --release
cp target/release/cdx ~/.local/bin/cdx
```

If the copy fails with **"file in use"**, the user has `cdx` open — ask them to close it first (Ctrl+C / Ctrl+Q), then retry.

## CI and branch protection

CI lives in `.github/workflows/ci.yml` and runs on **every PR** and **every push
to `master`** (no `paths` filters: a skipped workflow leaves the required checks
pending forever and deadlocks every PR). Three jobs:

- **`Build`**: `cargo build --locked --all-targets`.
- **`Lint`**: `cargo clippy --locked --all-targets -- -D warnings` (warnings are errors).
- **`Test`**: `cargo test --locked`.

`--locked` is intentional: a `Cargo.lock` that drifts from `Cargo.toml` fails
loudly instead of silently re-resolving dependencies.

Rules for the `master` branch (ruleset **`protect-master`**, reproducible with
`scripts/setup-repo-protection.sh`):

- Merge **only via PR**, with the three checks green; force-push and deletion of
  `master` are blocked.
- There is an **admin bypass** and it is **deliberate**: an admin *could* push
  directly, but the working intent is always the PR path. No non-admin actor can.
- `delete_branch_on_merge=true`: GitHub deletes the remote branch on merge.

Re-apply protection with `scripts/setup-repo-protection.sh` (idempotent; supports
`--dry-run`). It derives the default branch from the repo, so it is not tied to
`main`.
