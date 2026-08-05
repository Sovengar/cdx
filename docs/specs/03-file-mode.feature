Feature: File Search Mode
  As a user
  I want to search for files recursively from the current directory
  So that I can find and navigate to any file

  Background:
    Given the TUI is open
    And the current mode is "Files"

  # ── File listing ────────────────────────────────────────────────

  Scenario: Files are listed recursively from cwd
    Given the current directory is "~/dev/project"
    When the list loads
    Then all files under "~/dev/project" are shown recursively

  Scenario: Maximum depth is enforced
    Given max_secondary_depth is 5
    When the list loads
    Then files deeper than 5 levels are not included

  Scenario: exclude_dirs filter applies to files
    Given "node_modules" is in exclude_dirs
    When the list loads
    Then files inside "node_modules/" are not shown

  Scenario: show_dotfiles filter applies to files
    Given show_dotfiles is false
    When the list loads
    Then files starting with "." are not shown

  Scenario: show_winhidden filter applies to files
    Given show_winhidden is false
    When the list loads
    Then files inside "AppData/" are not shown (Windows only)

  Scenario: .ignore file rules apply to file listing
    Given a ".ignore" file exists in the project
    When the list loads
    Then files matching .ignore rules are excluded

  Scenario: Files display shows filename only
    Given a file at "~/dev/project/src/main.rs"
    When it appears in the list
    Then the display text is "main.rs"
    And the full path is stored internally

  Scenario: Fuzzy match filters file results
    Given 100 files are listed
    When I type "main" in the search bar
    Then only files matching "main" fuzzily are shown
    And results are sorted by nucleo score

  Scenario: Files mode does not show directories
    Given the current directory contains both files and subdirectories
    When the list loads
    Then only files are shown
    And directories are excluded from the list
