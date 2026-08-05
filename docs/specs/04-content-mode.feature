Feature: Content Search Mode (Grep)
  As a user
  I want to search file contents using ripgrep
  So that I can find where specific text appears in my codebase

  Background:
    Given the TUI is open
    And the current mode is "Content"

  # ── Grep execution ──────────────────────────────────────────────

  Scenario: Grep uses external rg process
    Given I type "TODO" in the search bar
    When the debounce (grep_debounce_ms) elapses
    Then the command `rg --json --smart-case TODO <root>` is executed
    And results are parsed from JSON output

  Scenario: Grep falls back to --vimgrep for old rg versions
    Given rg does not support --json (version < 13.0)
    When a grep search runs
    Then rg is re-invoked with --vimgrep
    And results are parsed from vimgrep format

  Scenario: Grep respects grep_max_depth
    Given grep_max_depth is 5
    When a grep search runs
    Then `--max-depth 5` is passed to rg

  Scenario: Grep applies exclude_dirs via --glob
    Given "node_modules" is in exclude_dirs
    When a grep search runs
    Then `--glob !node_modules` is passed to rg

  Scenario: Grep applies exclude_path_globs via --glob
    Given "**/go/pkg/mod" is in exclude_path_globs
    When a grep search runs
    Then `--glob !**/go/pkg/mod` is passed to rg

  Scenario: Grep applies show_dotfiles via --glob
    Given show_dotfiles is false
    When a grep search runs
    Then `--glob !.*` is passed to rg

  Scenario: Grep applies show_winhidden via --glob
    Given show_winhidden is false
    When a grep search runs
    Then `--glob !AppData` and `--glob !ProgramData` are passed to rg

  Scenario: Grep results show file:line format
    Given rg found "TODO" at line 42 in "src/main.rs"
    When the result appears in the list
    Then the display is "src/main.rs:42"
    And the full path points to the file

  Scenario: Grep search root is current directory
    Given the current directory is "~/dev/project"
    When a grep search runs
    Then rg searches from "~/dev/project" downward

  Scenario: Grep is blocking (not async)
    Given a grep search is in progress
    When I type additional characters
    Then the current grep search continues to completion
    And a new grep is triggered after debounce for the new query

  Scenario: WinHidden is auto-disabled in grep mode
    Given show_winhidden was true
    When I switch to Content mode
    Then show_winhidden becomes false
    And a warning message is printed to stderr
