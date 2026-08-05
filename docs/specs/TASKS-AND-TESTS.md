# Task Breakdown & Test Strategy

## Overview

This document breaks down the verification of the cdx-rs spec into actionable tasks,
each with a clear test strategy. Tasks are grouped by feature area and ordered by
dependency (foundational tasks first).

---

## Phase 1: Core Infrastructure (no UI dependency)

### Task 1.1: Config parsing and defaults
**Spec:** `11-config.feature`
**What to verify:**
- `config::init()` loads `~/.config/cdx/config.toml`
- Missing file → copies `config.default.toml`
- Invalid TOML → falls back to defaults + prints error
- All defaults match spec values

**Tests:**
```rust
#[test]
fn test_config_defaults() {
    config::init();
    let cfg = config::get();
    assert_eq!(cfg.find_debounce_ms, 50);
    assert_eq!(cfg.grep_debounce_ms, 80);
    assert_eq!(cfg.grep_max_depth, 5);
    assert_eq!(cfg.max_secondary_depth, 5);
    assert_eq!(cfg.zoxide_limit, 5);
    assert!(cfg.exclude_dirs.contains(&"node_modules"));
    assert!(cfg.exclude_dirs.contains(&".git"));
    assert!(cfg.exclude_win_dirs.contains(&"AppData"));
}

#[test]
fn test_config_keybinding_parsing() {
    let kb = KeybindingsRaw::default();
    let parsed = Keybindings::from_raw(&kb);
    assert_eq!(parsed.quit.code, KeyCode::Char('c'));
    assert!(parsed.quit.modifiers.contains(KeyModifiers::CONTROL));
    assert_eq!(parsed.switch_mode.code, KeyCode::Tab);
    assert_eq!(parsed.open_explorer.code, KeyCode::Char('o'));
}

#[test]
fn test_tool_selector_defaults() {
    config::init();
    let cfg = config::get();
    assert_eq!(cfg.tool_selector.len(), 4);
    assert_eq!(cfg.tool_selector[0].name, "yazi");
    assert_eq!(cfg.tool_selector[1].name, "nvim");
    assert_eq!(cfg.tool_selector[2].name, "opencode");
    assert_eq!(cfg.tool_selector[3].name, "lazygit");
}
```

---

### Task 1.2: Walker — list_dirs (sin query)
**Spec:** `02-directory-mode.feature` (sin query scenarios)
**What to verify:**
- `list_dirs()` uses `read_dir()` (1 level)
- Applies `should_exclude()` with all 3 filters
- Results sorted alphabetically
- `.ignore` does NOT affect results

**Tests:**
```rust
#[test]
fn test_list_dirs_respects_exclude_dirs() {
    // Setup: create temp dir with "node_modules" and "projects"
    // Call list_dirs()
    // Assert: "node_modules" absent, "projects" present
}

#[test]
fn test_list_dirs_respects_show_dotfiles() {
    // Setup: temp dir with ".hidden" and "visible"
    // list_dirs(root, false, false) → .hidden absent
    // list_dirs(root, true, false)  → .hidden present
}

#[test]
fn test_list_dirs_ignores_dotignore() {
    // Setup: temp dir with .ignore containing "Dropbox/"
    // Create "Dropbox" dir
    // list_dirs() → "Dropbox" IS present (read_dir doesn't read .ignore)
}

#[test]
fn test_list_dirs_sorted_alphabetically() {
    // Setup: temp dir with ["zebra", "alpha", "beta"]
    // list_dirs() → ["alpha", "beta", "zebra"]
}
```

---

### Task 1.3: Walker — recursive_dir_search (con query)
**Spec:** `02-directory-mode.feature` (con query scenarios)
**What to verify:**
- Uses `WalkBuilder` with `max_secondary_depth`
- Applies `entry_filter()` with exclude_dirs + dotfiles + winhidden
- Respects `.ignore` rules
- Cancellation via `AtomicBool` breaks the walk
- Results are relative to root

**Tests:**
```rust
#[test]
fn test_recursive_dir_search_max_depth() {
    // Setup: temp dir with nested dirs up to depth 8
    // Set max_secondary_depth = 5
    // recursive_dir_search() → only dirs up to depth 5
}

#[test]
fn test_recursive_dir_search_cancellation() {
    // Setup: large temp dir tree
    // Start search, immediately set cancel flag
    // Assert: results are empty or partial (early exit)
}

#[test]
fn test_recursive_dir_search_excludes_dirs() {
    // Setup: temp dir with "target/" subtree
    // recursive_dir_search() → no "target" entries
}
```

---

### Task 1.4: Walker — list_files
**Spec:** `03-file-mode.feature`
**What to verify:**
- Uses `WalkBuilder` with `max_secondary_depth`
- Only returns files (not directories)
- Applies all filters
- Display is filename only

**Tests:**
```rust
#[test]
fn test_list_files_only_files() {
    // Setup: temp dir with files and subdirs
    // list_files() → only files, no dirs
}

#[test]
fn test_list_files_max_depth() {
    // Setup: files at depth 1, 3, 6
    // max_secondary_depth = 5
    // list_files() → depth 1 and 3 only
}

#[test]
fn test_list_files_display_is_filename() {
    // Setup: file at "src/main.rs"
    // list_files() → display = "main.rs"
}
```

---

### Task 1.5: Grep — execute_search
**Spec:** `04-content-mode.feature`
**What to verify:**
- Calls `rg` with correct flags
- Applies all --glob filters
- Falls back to --vimgrep for old rg
- Results show file:line format

**Tests:**
```rust
#[test]
fn test_rg_command_args() {
    // Mock or inspect the Command built by run_rg_command()
    // Assert: --json, --smart-case, --max-depth, --glob flags
}

#[test]
fn test_grep_result_format() {
    // Setup: create file with known content
    // execute_search("pattern", root, ...)
    // Assert: results have format "relative/path:line_number"
}

#[test]
fn test_grep_fallback_vimgrep() {
    // This is harder to unit test — integration test with old rg
    // or mock the output detection logic
}
```

---

### Task 1.6: Zoxide integration
**Spec:** `02-directory-mode.feature` (zoxide scenarios)
**What to verify:**
- `get_list()` calls `zoxide query --list`
- `merge_with_dirs()` limits to `zoxide_limit`
- Excludes current dir and dirs already in walker results
- Prepends `~` for home-relative paths

**Tests:**
```rust
#[test]
fn test_zoxide_merge_limits() {
    let zoxide_cache = vec![PathBuf::from("/a"), PathBuf::from("/b"), PathBuf::from("/c")];
    let walker_items = vec![];
    let result = merge_with_dirs(&zoxide_cache, walker_items, &PathBuf::from("/"), 2);
    assert!(result.len() <= 2);
}

#[test]
fn test_zoxide_merge_excludes_current_dir() {
    let zoxide_cache = vec![PathBuf::from("/home/user/dev")];
    let walker_items = vec![];
    let result = merge_with_dirs(&zoxide_cache, &walker_items, &PathBuf::from("/home/user/dev"), 5);
    assert!(result.is_empty());
}

#[test]
fn test_zoxide_merge_excludes_duplicates() {
    let zoxide_cache = vec![PathBuf::from("/home/user/projects")];
    let walker_items = vec![DirEntryItem {
        display: "projects".into(),
        rel_path: "projects".into(),
        full_path: PathBuf::from("/home/user/projects"),
        is_zoxide: false,
        is_dir: true,
    }];
    let result = merge_with_dirs(&zoxide_cache, walker_items, &PathBuf::from("/home/user"), 5);
    // Should only appear once
    assert_eq!(result.iter().filter(|i| i.display == "projects").count(), 1);
}
```

---

## Phase 2: Search State & Async

### Task 2.1: Debounce logic
**Spec:** `01-search-bar.feature` (debounce scenarios)
**What to verify:**
- `set_deadline()` sets `Instant::now() + debounce_ms`
- `is_due()` returns true only after deadline
- `poll_timeout()` returns correct remaining time
- New keypress resets the deadline

**Tests:**
```rust
#[test]
fn test_debounce_set_and_check() {
    let mut state = SearchState::new(50);
    assert!(!state.is_due()); // no deadline set

    state.set_deadline("query");
    assert!(!state.is_due()); // just set, not yet due

    // After sleep(60ms) or mock time
    std::thread::sleep(Duration::from_millis(60));
    assert!(state.is_due());
}

#[test]
fn test_debounce_resets_on_new_query() {
    let mut state = SearchState::new(50);
    state.set_deadline("a");
    std::thread::sleep(Duration::from_millis(30));
    state.set_deadline("ab"); // resets
    assert!(!state.is_due()); // 30ms < 50ms from reset
}

#[test]
fn test_empty_query_clears_deadline() {
    let mut state = SearchState::new(50);
    state.set_deadline("query");
    state.set_deadline("");
    assert!(state.find_deadline.is_none());
}
```

---

### Task 2.2: Cancellation and generation counter
**Spec:** `01-search-bar.feature` (cancellation scenarios)
**What to verify:**
- `cancel()` sets AtomicBool to true
- `spawn_search()` creates new AtomicBool
- Generation incremented on each query change
- Stale results (wrong generation) are discarded

**Tests:**
```rust
#[test]
fn test_cancel_sets_flag() {
    let mut state = SearchState::new(50);
    let cancel = Arc::new(AtomicBool::new(false));
    state.cancel = Some(Arc::clone(&cancel));

    state.cancel();
    assert!(cancel.load(Ordering::Relaxed));
}

#[test]
fn test_generation_increment() {
    let mut state = SearchState::new(50);
    let gen_before = state.generation;
    state.generation = state.generation.wrapping_add(1);
    assert_eq!(state.generation, gen_before + 1);
}

#[test]
fn test_stale_results_ignored() {
    // Simulate: generation=5 results arrive when generation=6
    // receive_results() should return false
}
```

---

### Task 2.3: Score / fuzzy matching
**Spec:** `05-list-zone.feature` (fuzzy match scenarios)
**What to verify:**
- Uses `nucleo_matcher` with `CaseMatching::Ignore` + `Normalization::Smart`
- Scores items and sorts by relevance
- Top results appear first

**Tests:**
```rust
#[test]
fn test_score_items_exact_match_ranks_higher() {
    let items = vec![
        make_item("project-alpha"),
        make_item("my-project"),
        make_item("project"),
    ];
    let scored = score::score_items("project", &items, &mut matcher, &mut scratch);
    // "project" (exact) should rank first
    assert_eq!(scored[0].0, 2); // index of "project"
}

#[test]
fn test_score_items_case_insensitive() {
    let items = vec![make_item("README.md")];
    let scored = score::score_items("readme", &items, &mut matcher, &mut scratch);
    assert!(!scored.is_empty());
}
```

---

## Phase 3: TUI App State

### Task 3.1: Mode switching
**Spec:** `09-tab-behavior.feature`
**What to verify:**
- Tab cycles: Find → Search → Grep → Find
- Cancel + invalidate on switch
- Grep auto-disables winhidden
- Items refresh after switch

**Tests:**
```rust
#[test]
fn test_mode_cycle() {
    let mut app = App::new(None).unwrap();
    assert_eq!(app.mode, Mode::Find);

    app.switch_mode();
    assert_eq!(app.mode, Mode::Search);

    app.switch_mode();
    assert_eq!(app.mode, Mode::Grep);

    app.switch_mode();
    assert_eq!(app.mode, Mode::Find);
}

#[test]
fn test_grep_mode_disables_winhidden() {
    let mut app = App::new(None).unwrap();
    app.show_winhidden = true;
    app.switch_mode(); // → Search
    app.switch_mode(); // → Grep
    assert!(!app.show_winhidden);
}
```

---

### Task 3.2: Enter behavior
**Spec:** `07-enter-behavior.feature`
**What to verify:**
- Find mode: navigate to dir, clear query, invalidate cache
- Search/Grep mode: navigate to parent of file
- Preview Enter: navigate to subdirectory

**Tests:**
```rust
#[test]
fn test_enter_find_mode_navigates_to_dir() {
    let mut app = setup_app_with_dirs();
    app.list_state.select(Some(0));
    app.handle_enter();
    // Assert: current_dir changed, query cleared
}

#[test]
fn test_enter_search_mode_navigates_to_parent() {
    let mut app = setup_app_with_files();
    app.mode = Mode::Search;
    app.list_state.select(Some(0));
    app.handle_enter();
    // Assert: current_dir = parent of selected file
    assert_eq!(app.mode, Mode::Find); // switches back
}
```

---

### Task 3.3: Esc behavior
**Spec:** `08-esc-behavior.feature`
**What to verify:**
- Single Esc: navigate to parent
- Double Esc (< 300ms): navigate to home
- Esc in preview: return focus to list

**Tests:**
```rust
#[test]
fn test_single_esc_navigates_to_parent() {
    let mut app = App::new(None).unwrap();
    app.current_dir = PathBuf::from("/home/user/dev/projects");
    app.handle_esc();
    assert_eq!(app.current_dir, PathBuf::from("/home/user/dev"));
}

#[test]
fn test_double_esc_navigates_to_home() {
    let mut app = App::new(None).unwrap();
    app.current_dir = PathBuf::from("/home/user/dev/projects/deep");
    app.handle_esc(); // first
    // Simulate rapid second Esc (< 300ms)
    app.handle_esc();
    assert_eq!(app.current_dir, dirs::home_dir().unwrap());
}

#[test]
fn test_esc_in_preview_returns_to_list() {
    let mut app = App::new(None).unwrap();
    app.focus = Focus::Preview;
    // Simulate Esc key event
    // Assert: app.focus == Focus::List
}
```

---

### Task 3.4: Tool selector
**Spec:** `10-ctrl-keybindings.feature` (Ctrl+O scenarios)
**What to verify:**
- Opens popup with tool list
- Navigate with Up/Down/j/k
- Enter spawns tool inline
- Esc/q closes popup

**Tests:**
```rust
#[test]
fn test_tool_selector_opens() {
    let mut app = setup_app_with_selected_dir();
    execute_action(&mut app, "open_explorer");
    assert_eq!(app.popup, Some(Popup::ToolSelector));
    assert_eq!(app.popup_index, 0);
}

#[test]
fn test_tool_selector_navigation() {
    let mut app = setup_app_with_tool_selector_open();
    // Simulate Down key
    assert_eq!(app.popup_index, 1);
    // Simulate Up key
    assert_eq!(app.popup_index, 0);
}
```

---

## Phase 4: UI Rendering

### Task 4.1: List rendering
**Spec:** `05-list-zone.feature`
**What to verify:**
- Icons applied correctly (zoxide=★, dir=folder, file=extension icon)
- Styles applied (zoxide=yellow bold, dir=cyan bold)
- Counter shows filtered/total
- Mode label changes (Dirs/Files/Content)

**Tests:** (visual/integration — manual or snapshot)
```rust
#[test]
fn test_list_item_icons() {
    let zoxide = DirEntryItem { is_zoxide: true, .. };
    assert_eq!(icon_for_item(&zoxide), "★");

    let dir = DirEntryItem { is_dir: true, .. };
    assert_eq!(icon_for_item(&dir), ""); // folder icon in code
}

#[test]
fn test_list_item_styles() {
    let zoxide = DirEntryItem { is_zoxide: true, .. };
    let style = style_for_item(&zoxide);
    // Assert: fg = Yellow, modifier includes BOLD
}
```

---

### Task 4.2: Preview rendering
**Spec:** `06-preview-zone.feature`
**What to verify:**
- Tree renders with ├── / └── connectors
- Git status section present for dirs
- eza output below tree
- bat output for files
- Scroll works (mouse + keyboard)
- Auto-scroll to keep selection visible

**Tests:** (integration — run with known directory structure)
```rust
#[test]
fn test_preview_directory_has_tree_and_git() {
    let app = setup_app();
    let item = make_dir_item("some-repo");
    let text = preview::generate(&app, &item);
    let content = format!("{:?}", text);
    assert!(content.contains("GIT STATUS"));
}

#[test]
fn test_preview_file_uses_bat() {
    // Verify preview_file calls bat
    // Fallback to fs::read_to_string if bat missing
}
```

---

### Task 4.3: Status bar & input rendering
**Spec:** general UI
**What to verify:**
- Path displayed with ~ prefix for home
- Mode indicator (Dirs/Files/Content)
- Dotfiles indicator (✓/✗)
- Spinner shown during active search
- Input shows "> " prefix with cursor

---

## Phase 5: Integration / E2E

### Task 5.1: Full flow — directory navigation
**Spec:** combined `02`, `07`, `08`
**Test scenario:**
1. Start in `~/dev`
2. Verify zoxide + children shown
3. Type "proj" → verify async search returns results
4. Enter on "projects" → verify navigation
5. Esc → verify parent navigation
6. Double Esc → verify home navigation

### Task 5.2: Full flow — mode switching
**Spec:** combined `03`, `04`, `09`
**Test scenario:**
1. Start in Dirs mode, empty query
2. Tab → Files mode, verify file list
3. Type "main" → verify fuzzy filter
4. Tab → Content mode, verify grep runs
5. Type "fn " → verify grep results
6. Tab → back to Dirs mode

### Task 5.3: Full flow — tool spawning
**Spec:** combined `10`
**Test scenario:**
1. Navigate to a directory
2. Ctrl+O → verify popup appears
3. Navigate to "nvim" → Enter
4. Verify TUI pauses, nvim opens
5. Exit nvim → verify TUI resumes

### Task 5.4: Cancellation under load
**Spec:** `01-search-bar.feature`
**Test scenario:**
1. Navigate to a large directory tree
2. Type rapidly: "a", "b", "c", "d" within 50ms
3. Verify only final query search executes
4. Verify no stale results appear

---

## Test Classification

| Category | Count | Approach |
|----------|-------|----------|
| **Unit tests** (Rust `#[test]`) | ~30 | Config parsing, walker functions, debounce, scoring, zoxide merge |
| **Integration tests** | ~10 | App state transitions, mode switching, enter/esc behavior |
| **E2E / Manual** | ~5 | Full TUI flow with real terminal, tool spawning |
| **Visual / Snapshot** | ~5 | UI rendering correctness (icons, styles, layout) |

---

## Priority Order

1. **P0 — Must pass before any change:**
   - Config defaults (1.1)
   - Walker functions (1.2, 1.3, 1.4)
   - Debounce logic (2.1)
   - Cancellation (2.2)

2. **P1 — Core behavior:**
   - Mode switching (3.1)
   - Enter/Esc behavior (3.2, 3.3)
   - Fuzzy scoring (2.3)
   - Grep execution (1.5)

3. **P2 — UI & polish:**
   - List rendering (4.1)
   - Preview rendering (4.2)
   - Tool selector (3.4)

4. **P3 — Integration:**
   - Full flow tests (5.1–5.4)
   - Cancellation under load (5.4)
