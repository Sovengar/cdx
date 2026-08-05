Feature: Tab Key Behavior
  As a user
  I want Tab to cycle between search modes
  So that I can switch between searching directories, files, and content

  Background:
    Given the TUI is open

  # ── Mode cycling ────────────────────────────────────────────────

  Scenario: Tab cycles from Directories to Files
    Given the current mode is "Directories"
    When I press Tab
    Then the mode changes to "Files"
    And the list refreshes with file results
    And the search query is re-applied

  Scenario: Tab cycles from Files to Content
    Given the current mode is "Files"
    When I press Tab
    Then the mode changes to "Content"
    And the list refreshes with grep results (if query exists)
    And the search query is re-applied

  Scenario: Tab cycles from Content back to Directories
    Given the current mode is "Content"
    When I press Tab
    Then the mode changes to "Directories"
    And the list refreshes with directory results
    And the search query is re-applied

  Scenario: Tab cancels any in-progress search
    Given a directory search is running in background
    When I press Tab
    Then the running search is cancelled
    And the new mode's search starts fresh

  Scenario: Tab resets find cache
    Given the find cache is loaded for the current directory
    When I press Tab
    Then the find cache is invalidated
    And a fresh search is performed for the new mode

  Scenario: Tab in preview also cycles mode
    Given focus is on the preview zone
    When I press Tab
    Then focus returns to the list
    And the mode cycles to the next one
    And the list refreshes
