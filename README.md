# cdx — interactive directory navigator

Jump between directories faster than `cd`. Think `zoxide` meets `fzf` in a TUI but with steroids + superpowers + wisdom of _Platon_.

![screenshot](https://img.shields.io/badge/status-beta-blue)

## Features

See [FEATURES.md](FEATURES.md) for the full list.

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
# Binary at target/release/cdx
```

On Windows, copy to `~/.local/bin/cdx.exe`:

```powershell
Copy-Item target/release/cdx.exe ~/.local/bin/cdx.exe
```

On Unix:

```bash
cp target/release/cdx ~/.local/bin/cdx
```

Make sure `~/.local/bin` is in your PATH.

## Wrappers

Wrapping the output of the TUI is mandatory to complete the navigation. It is done in the wrapper so you can also customize the end result, like for example using `eza` after navigation.

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

### Fish wrapper

Add to `~/.config/fish/functions/cdx.fish`:

```fish
function cdx --wraps cdx --description 'Interactive directory navigator wrapper'
    set -l result_file /tmp/cdx-result.txt
    rm -f $result_file
    command cdx $argv
    if test $status -eq 0 -a -f "$result_file"
        set -l target (string trim (cat $result_file))
        rm -f $result_file
        if test -n "$target" -a -d "$target"
            builtin cd "$target"
            command eza --icons --group-directories-first 2>/dev/null; or ls --color=auto
        end
    end
end
```

## Usage

| Command             | Action                        |
| ------------------- | ----------------------------- |
| `cdx`               | Open TUI at current directory |
| `cdx <path>`        | Jump to path                  |
| `cdx <name>`        | Jump via zoxide               |
| `cdx -g <query>`    | Global content search         |
| `cdx ~` / `cdx ...` | Print HOME path               |

### TUI keybindings

All keybindings are customizable in `~/.config/cdx/config.toml` under `[keys]`.

## Dependencies

- **Runtime:** `rg` (ripgrep), `zoxide`, `fzf` (for `-g` mode), `yazi` (for `Ctrl+Enter`)
- **Optional:** `eza`, `bat` (for enhanced preview)

## Configuration

`~/.config/cdx/config.toml` is auto-generated on first run. Edit with `Ctrl+E` from the TUI.

See [config.default.toml](config.default.toml) for all available options.

## License

MIT
