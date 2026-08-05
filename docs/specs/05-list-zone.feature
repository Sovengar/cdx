Feature: List Zone Display
  As a user
  I want the list to show search results in a clear, relative format
  So that I can quickly identify and select items

  Background:
    Given the TUI is open
    And the list zone is visible

  # ── Display format ──────────────────────────────────────────────

  Scenario: Directory items show relative path from cwd
    Given the current directory is "~/dev"
    And a directory "~/dev/projects/myProject" exists
    When it appears in the list
    Then the display shows "projects/myProject"

  Scenario: Zoxide items show full path with ~ prefix
    Given a zoxide entry is "/home/user/.config/nvim"
    When it appears in the list
    Then the display shows "~/.config/nvim"

  Scenario: File items show filename only
    Given a file at "~/dev/project/src/main.rs"
    When it appears in the list (File mode)
    Then the display shows "main.rs"

  Scenario: Content items show relative_path:line_number
    Given a grep match at "src/lib.rs" line 15
    When it appears in the list
    Then the display shows "src/lib.rs:15"

  Scenario: Directories (sin query) show cwd children sorted
    Given the current directory has entries: ["zebra", "alpha", "beta"]
    When the list loads
    Then items are sorted alphabetically: alpha, beta, zebra

  Scenario: Items have appropriate icons
    Given a zoxide item is in the list
    Then it shows a star icon (★)
    Given a directory item is in the list
    Then it shows a folder icon
    Given a .rs file is in the list
    Then it shows the Rust icon

  Scenario: Items have appropriate colors
    Given a zoxide item is in the list
    Then it is styled yellow and bold
    Given a directory item is in the list
    Then it is styled cyan and bold
    Given a .rs file is in the list
    Then it is styled light blue

  # ── Selection ───────────────────────────────────────────────────

  Scenario: First item is auto-selected when results appear
    Given search results are loaded
    When the list renders
    Then the first item is highlighted/selected

  Scenario: Counter shows filtered/total
    Given 10 items match out of 50 total
    When the list renders
    Then the header shows "10/50"
