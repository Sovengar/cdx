# cdx-rs

Interactive directory navigator — Rust rewrite of Cdx.ps1.

## Installation

```powershell
# Build
cd ~/dev/cdx-rs
cargo build --release

# Install to PATH
cp target/release/cdx-rs.exe ~/.local/bin/
```

The PowerShell profile already includes the `cdx` wrapper function. Source it:

```powershell
. $PROFILE
```

## Usage

| Command | Action |
|---------|--------|
| `cdx` | Open TUI browser at current directory |
| `cdx <path>` | Jump to path |
| `cdx <name>` | Jump via zoxide |
| `cdx -g <query>` | Global content search (3-phase rg) |
| `cdx -h` | Show help |
| `cdx ~` / `cdx ...` | Print HOME path |
| `Alt+C` | Launch cdx TUI from anywhere (PSReadLine) |

## Keybindings (TUI)

| Key | Action |
|-----|--------|
| `Enter` | cd into directory / open file |
| `Esc` | Go to parent directory |
| `↑` / `↓` | Navigate list |
| `Tab` | Toggle focus: List ↔ Input |
| `Ctrl+C` | Exit (stay in current dir) |
| `Ctrl+G` | Cycle modes: Find → Search → Grep → Find |
| `Ctrl+A` | Toggle dotfiles |
| `Ctrl+W` | Toggle WinHidden directories |
| `Ctrl+H` | Go to HOME |
| `Ctrl+O` | Open selected dir in yazi |

## Dependencies

- **Runtime:** `rg` (ripgrep), `zoxide`, `fzf` (for `-g` mode)
- **Optional:** `eza`, `bat` (for enhanced preview)
