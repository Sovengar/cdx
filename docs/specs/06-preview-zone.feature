Feature: Preview Zone
  As a user
  I want to preview the selected item's details before navigating
  So that I can make informed decisions about where to go

  Background:
    Given the TUI is open
    And the preview zone is visible on the right side

  # ── Access ──────────────────────────────────────────────────────

  Scenario: Enter preview from list with Right arrow
    Given an item is selected in the list
    When I press the Right arrow key
    Then focus moves to the preview zone
    And the preview content is displayed

  Scenario: Return to list from preview with Left arrow
    Given focus is on the preview zone
    When I press the Left arrow key
    Then focus returns to the list zone

  Scenario: Return to list from preview with Esc
    Given focus is on the preview zone
    When I press Esc
    Then focus returns to the list zone

  # ── Directory preview content ───────────────────────────────────

  Scenario: Directory preview shows tree structure
    Given a directory with subdirectories is selected
    When the preview renders
    Then a tree view is shown with ├── and └── connectors
    And directories are shown in cyan bold
    And files are shown with appropriate icons

  Scenario: Directory preview shows git status
    Given the selected directory is a git repository
    When the preview renders
    Then "=== GIT STATUS ===" header is shown
    And `git status --short` output is displayed
    And modified/untracked files are highlighted in red

  Scenario: Directory preview shows eza output
    Given the selected directory has contents
    When the preview renders
    Then `eza --icons=always --color=always --group-directories-first --grid --width=40` is shown
    And eza output is displayed below the tree

  Scenario: Directory preview shows tree with max depth 2
    Given the selected directory has deep nesting
    When the preview renders
    Then the tree shows at most 2 levels of depth
    And deeper items show "⋯ (N more)" if truncated

  Scenario: Directory preview tree trims files to 3
    Given a directory has more than 3 files
    When the preview renders
    Then only 3 files are shown
    And a "⋯ (N more)" line indicates the count

  # ── File preview content ────────────────────────────────────────

  Scenario: File preview shows bat output
    Given a text file is selected
    When the preview renders
    Then `bat --color=always --line-range :50 --paging=never` output is shown
    And at most 50 lines are displayed

  Scenario: File preview falls back to raw read
    Given bat is not installed
    When a file is selected
    Then the preview shows the first 50 lines via fs::read_to_string

  # ── Content (grep) preview ─────────────────────────────────────

  Scenario: Grep preview shows rg context
    Given a grep result for "src/main.rs:42" is selected
    When the preview renders
    Then `rg --context=2 --color=never --max-count 50` is executed on the file
    And matching lines and context are shown
    And separator lines (--) are styled dark gray

  Scenario: Grep preview handles no matches
    Given a grep result file has no matches (file changed)
    When the preview renders
    Then "(no matches in <path>)" is displayed

  # ── Navigation in preview ──────────────────────────────────────

  Scenario: Navigate into subdirectory from preview
    Given focus is on the preview zone
    And a directory entry is selected in the preview
    When I press Enter
    Then current_dir changes to that subdirectory
    And focus returns to the list
    And the list refreshes with the new directory contents

  Scenario: Up/Down arrows navigate preview entries
    Given focus is on the preview zone
    And the preview has multiple directory entries
    When I press Up/Down arrows
    Then the selection moves between entries
    And the selected entry is highlighted with REVERSED style

  # ── Scroll ──────────────────────────────────────────────────────

  Scenario: Mouse scroll works in preview
    Given the preview has content exceeding the visible area
    When I scroll the mouse wheel down
    Then the preview scrolls down by 3 lines

  Scenario: PageUp/PageDown scroll the preview
    Given the preview has content exceeding the visible area
    When I press PageDown
    Then the preview scrolls down by 10 lines
    When I press PageUp
    Then the preview scrolls up by 10 lines

  Scenario: Preview scroll resets on item change
    Given I have scrolled down in the preview
    When I select a different item in the list
    Then the preview scroll resets to 0
