# cdx-rs — Implementation Tasks

## P1: Skeleton

- [ ] **T1.1** — Create Cargo.toml with all dependencies
  - Module: `Cargo.toml`
  - Verifies: `cargo check` passes with all dependencies (ratatui, crossterm, nucleo-matcher, ignore, clap, dirs, anyhow, thiserror)

- [ ] **T1.2** — Implement CLI argument parsing
  - Module: `cli.rs`
  - Verifies: `cdx --help` shows help, `-g` flag recognized, query args captured

- [ ] **T1.3** — Define config constants (exclude lists)
  - Module: `config.rs`
  - Verifies: EXCLUDE_DIRS, EXCLUDE_WIN_DIRS, EXCLUDE_PATH_GLOBS, PRIORITY_ROOTS defined

- [ ] **T1.4** — Implement main.rs entry point with dispatch
  - Module: `main.rs`
  - Verifies: `cdx` runs TUI, `cdx somepath` prints path, `cdx ~` prints HOME, `cdx -g query` triggers search mode

---

## P2: Walker

- [ ] **T2.1** — Implement directory listing with ignore crate
  - Module: `walker/mod.rs` — `list_dirs()`
  - Verifies: Lists immediate subdirectories, respects max_depth(1), filters by file_type (dirs only)

- [ ] **T2.2** — Implement file listing
  - Module: `walker/mod.rs` — `list_files()`
  - Verifies: Lists immediate files (equivalent to `rg --files`)

- [ ] **T2.3** — Implement zoxide integration
  - Module: `zoxide/mod.rs` — `query()`, `get_list()`
  - Verifies: `zoxide query <name>` returns path, `zoxide query --list` returns all entries

- [ ] **T2.4** — Implement exclusion filters
  - Module: `walker/mod.rs` — `is_excluded()`
  - Verifies: node_modules, .git, .cache excluded; AppData/ProgramData excluded when show_winhidden=false; dotfiles excluded when show_dotfiles=false

- [ ] **T2.5** — Implement zoxide merge with walker items
  - Module: `walker/mod.rs` or `tui/app.rs`
  - Verifies: Zoxide entries marked with ★, appear first, deduplicated with walker items

---

## P3: TUI Core

- [ ] **T3.1** — Define App state struct and data types
  - Module: `tui/app.rs`
  - Verifies: App struct with current_dir, items, filtered_indices, list_state, Mode enum, DirEntryItem, Focus, ExitAction

- [ ] **T3.2** — Implement event loop with crossterm
  - Module: `tui/events.rs`
  - Verifies: `event::poll()` works, key events captured, terminal enters/exits raw mode

- [ ] **T3.3** — Implement basic UI rendering
  - Module: `tui/ui.rs` — `render()`, `render_list()`
  - Verifies: List displays items, navigation with arrow keys works, selection highlighted

- [ ] **T3.4** — Implement Enter/Esc navigation
  - Module: `tui/app.rs` — `handle_enter()`, `handle_esc()`
  - Verifies: Enter cd into directory, Esc goes to parent, query persists on Esc

---

## P4: TUI Interactive

- [ ] **T4.1** — Implement fuzzy filtering with nucleo
  - Module: `tui/app.rs` — `apply_query()`
  - Verifies: Typing filters list, selection resets to first item, empty query shows all items

- [ ] **T4.2** — Implement 3-way mode toggle (Ctrl+G)
  - Module: `tui/app.rs` — `switch_mode()`
  - Verifies: Find → Search → Grep → Find cycle, mode indicator updates

- [ ] **T4.3** — Implement toggles (Ctrl+A dotfiles, Ctrl+W WinHidden)
  - Module: `tui/app.rs` — `toggle_dotfiles()`, `toggle_winhidden()`
  - Verifies: Ctrl+A shows/hides dotfiles, Ctrl+W shows/hides Windows hidden dirs

- [ ] **T4.4** — Implement header and status bar
  - Module: `tui/ui.rs` — `render_header()`, `render_status()`
  - Verifies: Header shows shortcuts per mode, status shows path + mode + toggles state

- [ ] **T4.5** — Implement Ctrl+H (home) and Ctrl+O (yazi)
  - Module: `tui/app.rs` — `handle_key()`
  - Verifies: Ctrl+H jumps to HOME, Ctrl+O sets ExitAction::SpawnYazi

---

## P5: Preview

- [ ] **T5.1** — Implement directory preview (eza + git)
  - Module: `preview/mod.rs` — `preview_directory()`
  - Verifies: Shows "=== CONTENTS ===" with eza output, "=== GIT STATUS ===" with git status

- [ ] **T5.2** — Implement file preview (bat)
  - Module: `preview/mod.rs` — `preview_file()`
  - Verifies: Shows file content with bat, fallback to raw read if bat unavailable

- [ ] **T5.3** — Integrate preview into TUI
  - Module: `tui/ui.rs` — `render_preview()`, `tui/app.rs` — preview_dirty flag
  - Verifies: Preview panel updates on selection change, shows placeholder when no selection

---

## P6: Search + Grep

- [ ] **T6.1** — Implement global search (-g) with 3-phase rg
  - Module: `search/mod.rs` — `global_search()`
  - Verifies: Phase 1 content matches, Phase 2 filename matches, Phase 3 dirname matches, combined + deduplicated

- [ ] **T6.2** — Implement Grep mode (rg --vimgrep)
  - Module: `grep/mod.rs` — `parse_vimgrep()`, `tui/app.rs` — `run_grep_search()`
  - Verifies: rg --vimgrep output parsed into GrepMatch[], displayed as list items

- [ ] **T6.3** — Implement Grep debounce (300ms)
  - Module: `tui/events.rs` — event loop timeout
  - Verifies: Typing in Grep mode waits 300ms after stop before executing rg

- [ ] **T6.4** — Implement search root clamping
  - Module: `tui/app.rs` — `clamp_search_root()`
  - Verifies: Grep searches from HOME if current_dir outside HOME, auto-disables WinHidden on Grep entry

- [ ] **T6.5** — Implement Grep preview with context
  - Module: `grep/mod.rs` — `preview_match()`
  - Verifies: Selected match shows rg --context=2 output with surrounding lines

---

## P7: Polish

- [ ] **T7.1** — Add error handling with anyhow/thiserror
  - Module: All modules
  - Verifies: No panics, graceful error messages for missing dependencies (eza, bat, git, rg, zoxide)

- [ ] **T7.2** — Create PowerShell wrapper (cdx function)
  - Module: `Microsoft.PowerShell_profile.ps1` (via chezmoi)
  - Verifies: `cdx` calls binary, handles exit code, calls Show-CdxResult after cd

- [ ] **T7.3** — Add PSReadLine shortcut (Alt+C)
  - Module: `Microsoft.PowerShell_profile.ps1`
  - Verifies: Alt+C launches cdx, passes current buffer as query if present

- [ ] **T7.4** — Integrate with chezmoi
  - Module: chezmoi source
  - Verifies: `chezmoi add` captures wrapper, `chezmoi apply` deploys to profile

- [ ] **T7.5** — Write README.md documentation
  - Module: `README.md`
  - Verifies: Installation instructions, keybindings table, feature parity list

- [ ] **T7.6** — Build release binary and verify
  - Module: Release workflow
  - Verifies: `cargo build --release` produces <5MB binary, runs all modes without crash