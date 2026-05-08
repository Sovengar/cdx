# cdx — interactive directory navigator

Jump between directories faster than `cd`. Think `zoxide` meets `fzf` in a TUI.

![screenshot](https://img.shields.io/badge/status-beta-blue)

## Features

- **TUI browser** — Fuzzy-filter directories and files, preview contents inline
- **Three modes**: **Find** (dirs only), **Search** (files + dirs), **Grep** (full-text via ripgrep)
- **Zoxide integration** — Frequently-used paths show first with ★
- **Tree preview** — Navigate directory trees from the preview panel
- **Git awareness** — Shows branch, dirty/clean status, git status in preview
- **Fully configurable** — `~/.config/cdx/config.toml`, including keybindings
- **Explorer integration** — `Ctrl+Enter` to open selected dir in yazi (or your file manager)
- **Cross-platform** — Windows (PowerShell) and Unix

## Quick install

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Sovengar/cdx/main/scripts/install.ps1 | iex
```

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/Sovengar/cdx/main/scripts/install.sh | bash
```

## Manual install

Requires Rust: https://rustup.rs

```bash
git clone https://github.com/Sovengar/cdx.git
cd cdx
cargo build --release
# Binary at target/release/cdx-rs
```

On Windows, copy to `~/.local/bin/cdx.exe`:
```powershell
Copy-Item target/release/cdx.exe ~/.local/bin/cdx.exe
```

On Unix:
```bash
cp target/release/cdx-rs ~/.local/bin/cdx
```

Make sure `~/.local/bin` is in your PATH.

### PowerShell wrapper

Add this to your `$PROFILE` to make `cdx` change the shell's current directory:

```powershell
function cdx {
    $result = & "$env:USERPROFILE\.local\bin\cdx.exe" @args
    if ($LASTEXITCODE -eq 0 -and $result) {
        Set-Location $result
    }
}
```

For a PSReadLine keybinding (`Ctrl+Shift+G`) to launch cdx from anywhere:

```powershell
Set-PSReadLineKeyHandler -Key Ctrl+Shift+G -ScriptBlock {
    [Microsoft.PowerShell.PSConsoleReadLine]::Insert("cdx")
    [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
}
```

## Usage

| Command | Action |
|---------|--------|
| `cdx` | Open TUI at current directory |
| `cdx <path>` | Jump to path |
| `cdx <name>` | Jump via zoxide |
| `cdx -g <query>` | Global content search |
| `cdx ~` / `cdx ...` | Print HOME path |

### TUI keybindings

| Key | Action |
|-----|--------|
| `Enter` | cd into directory |
| `Esc` / `Esc²` | Go to parent / Go to HOME |
| `↑` / `↓` | Navigate list |
| `Tab` | Cycle mode: Find → Search → Grep |
| `Ctrl+Enter` | Open in yazi/explorer |
| `Ctrl+A` | Toggle dotfiles |
| `Ctrl+W` | Toggle WinHidden files |
| `Ctrl+E` | Edit config file |
| `Ctrl+C` | Quit |

All keybindings are customizable in `~/.config/cdx/config.toml` under `[keys]`.

## Dependencies

- **Runtime:** `rg` (ripgrep), `zoxide`, `fzf` (for `-g` mode), `yazi` (for `Ctrl+Enter`)
- **Optional:** `eza`, `bat` (for enhanced preview)

## Configuration

`~/.config/cdx/config.toml` is auto-generated on first run. Edit with `Ctrl+E` from the TUI.

See [config.default.toml](config.default.toml) for all available options.

## License

MIT
