# cdx-rs — Agent Instructions

## Build & Deploy

After making source changes, you MUST build the release binary and copy it to `~/.local/bin/` so the user can test it.

The `cdx` shell command calls `~/.local/bin/cdx.exe` (NOT `cdx-rs.exe`). Source changes alone won't be visible until the binary is deployed there.

```powershell
cargo build --release
Copy-Item -LiteralPath "target\release\cdx.exe" -Destination "$env:USERPROFILE\.local\bin\cdx.exe" -Force
```

If the copy fails with **"file in use"**, the user has `cdx` open — ask them to close it first (Ctrl+C / Ctrl+Q), then retry.
