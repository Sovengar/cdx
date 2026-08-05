# cdx-rs — Agent Instructions

## Build & Deploy

After making source changes, you MUST build the release binary and copy it to `~/.local/bin/` so the user can test it.

The `cdx` shell command calls `~/.local/bin/cdx`. Source changes alone won't be visible until the binary is deployed there.

```bash
cargo build --release
cp target/release/cdx ~/.local/bin/cdx
```

If the copy fails with **"file in use"**, the user has `cdx` open — ask them to close it first (Ctrl+C / Ctrl+Q), then retry.
