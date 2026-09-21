# Features

- **TUI browser** — Fuzzy-filter directories and files, preview contents inline
- **Three modes**: **Directories** (dirs only), **Files** (files + dirs), **Content** (full-text via ripgrep)
- **Zoxide integration** — Frequently-used paths show first with ★
- **Tree preview** — Navigate directory trees from the preview panel
- **Git awareness** — Shows branch, dirty/clean status, git status in preview
- **Fully configurable** — `~/.config/cdx/config.toml`, including keybindings
- **Explorer integration** — `Ctrl+O` to open selected item with yazi, nvim, opencode, lazygit (configurable)
- **Inline editor** — `Ctrl+E` opens selected file in nvim inline
- **Cross-platform** — Windows (PowerShell) and Unix

---

## CLI Syntax

```
cdx              TUI — navigate directories
cdx <path>       direct cd if path exists
cdx <name>       zoxide → fallback to TUI with query
cdx -g <query>   global content search (rg)
cdx -h           help
cdx ~            go to $HOME
cdx ...          go to $HOME
```

## Jump Mode (when given an argument)

1. `cdx <path>` → direct cd if the path exists
2. If not → `zoxide query` (frecency, if installed)
3. If no match → opens TUI with pre-filled query

---

## Search Bar

Async, non-blocking text input with debounce to prevent overload on every keypress. Includes cancellation system for stale searches after additional keypresses (add or remove characters).

|                  | **Directories (no query)** | **Directories (with query)** | **Files** | **Content**              |
| ---------------- | -------------------------- | ---------------------------- | --------- | ------------------------ |
| **Zoxide**       | Yes (first time only)      | No                           | No        | No                       |
| **Async**        | No                         | Yes (thread + channel)       | No        | No (blocking)            |
| **Debounce**     | No                         | Yes (`find_debounce_ms`)     | No        | Yes (`grep_debounce_ms`) |
| **Cancellation** | No                         | Yes (`AtomicBool`)           | No        | No                       |

## Search Types

All modes search from cwd forward.

|                      | **Directories (no query)**        | **Directories (with query)**                     | **Files**                             | **Content**                      |
| -------------------- | --------------------------------- | ------------------------------------------------ | ------------------------------------- | -------------------------------- |
| **What it shows**    | Direct children of cwd + 5 zoxide | Subdirectories matching the fuzzy query          | Files (not dirs) from cwd recursively | Lines containing the text        |
| **Function**         | `list_dirs()`                     | `recursive_dir_search()` → `filter_find_cache()` | `list_files()`                        | `run_grep_search()` → `rg`       |
| **Engine**           | `std::fs::read_dir()`             | `WalkBuilder` + nucleus matcher                  | `WalkBuilder`                         | External `rg` process            |
| **Depth**            | 1 level                           | `max_secondary_depth` (5)                        | `max_secondary_depth` (5)             | `grep_max_depth` (5)             |
| **`.ignore`**        | No                                | Yes                                              | Yes                                   | Yes (rg handles it)              |
| **`exclude_dirs`**   | Yes                               | Yes                                              | Yes                                   | Yes (via `--glob !dir`)          |
| **`show_dotfiles`**  | Yes                               | Yes                                              | Yes                                   | Yes (via `--glob !.*`)           |
| **`show_winhidden`** | Yes                               | Yes                                              | Yes                                   | Yes (via `--glob !AppData` etc.) |
| **Fuzzy match**      | No (shows all)                    | Yes (nucleus)                                    | Yes (nucleus)                         | No (rg filters)                  |

- **Directories (no query)**: Uses `read_dir()` with filters `exclude_dirs`, `exclude_win_dirs`, `show_dotfiles`.
- **Directories (with query)**: Uses async `recursive_dir_search()`.
- **Files**: Uses `WalkBuilder` with filters from `~/.ignore`, `exclude_dirs`, `exclude_win_dirs`, `show_dotfiles`. Max depth: 5 levels.
- **Content**: Uses `rg` (ripgrep) with its own filters: `~/.ignore`, `.gitignore`, `.rgignore`. Config's `exclude_dirs` and `show_dotfiles` translate to `--glob` args.

## List Zone

Displays fuzzy-matched results.

- First time opening, the 5 most-used zoxide dirs are merged in. After any search or tab switch, zoxide dirs are hidden.
- Paths are relative to cwd (e.g. in `~/dev`, searching shows `projects/myProject` instead of the full path).

|             | **Directories (no query)** | **Directories (with query)** | **Files** | **Content**     |
| ----------- | -------------------------- | ---------------------------- | --------- | --------------- |
| **Display** | Relative to cwd            | Relative to cwd              | Filename  | `relative:line` |

## Preview Zone

Accessed from list zone by pressing ←.

Infinite-scroll panel showing:

- Directory tree
- Git status
- `eza` output
- `bat` output
- `rg` context

Supports navigation into subdirectories.

---

## Keybinding behaviour

| Key        | In List                                                   | In Preview   |
| ---------- | --------------------------------------------------------- | ------------ |
| **Enter**  | Navigate to selected dir, clear search                    | —            |
| **Esc**    | cd .., re-run search if query active. Double Esc → ~      | Back to list |
| **Tab**    | Cycle modes: Files → Directories → Content, re-run search | —            |
| **Ctrl+O** | Open selector (yazi, nvim, opencode, lazygit) — inline    | —            |
| **Ctrl+E** | Open with nvim inline                                     | —            |
| **Ctrl+C** | Exit TUI, print cwd to console                            | —            |

Tools opened via Ctrl+O/Ctrl+E are **inline** — they pause the TUI, don't close it.
