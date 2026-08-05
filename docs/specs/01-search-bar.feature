Feature: Search Bar
  As a user navigating directories
  I want a search bar that accepts text input asynchronously
  So that I can find directories, files, or content without blocking the UI

  Background:
    Given the TUI is open
    And the search bar is visible and focused

  # ── Async behavior ──────────────────────────────────────────────

  Scenario: Search bar accepts input without blocking
    When I type "myproject" into the search bar
    Then the UI remains responsive during the search
    And the spinner indicator appears in the status bar

  Scenario: Directory search (con query) runs asynchronously
    Given the current mode is "Directories"
    When I type "proj" into the search bar
    Then the search executes in a background thread
    And results appear via channel when ready
    And the UI does not freeze

  Scenario: File search runs synchronously with debounce
    Given the current mode is "Files"
    When I type "config" into the search bar
    Then the search blocks briefly after debounce elapses
    And results appear after the search completes

  Scenario: Content search (grep) runs synchronously with debounce
    Given the current mode is "Content"
    When I type "TODO" into the search bar
    Then the grep command blocks after debounce elapses
    And results appear after rg finishes

  # ── Debounce ────────────────────────────────────────────────────

  Scenario: Debounce prevents overload on rapid keypresses
    Given the find debounce is set to 50ms
    When I type "a" then "b" then "c" within 30ms
    Then only one search is triggered for "abc"
    And no intermediate searches for "a" or "ab" occur

  Scenario: Grep debounce is independent from find debounce
    Given the grep debounce is set to 80ms
    And the find debounce is set to 50ms
    When I am in Content mode and type quickly
    Then the debounce delay uses grep_debounce_ms (80ms)

  Scenario: Find debounce applies to both Directories and Files modes
    Given the find debounce is set to 50ms
    When I am in Directories mode with a query
    Then the debounce uses find_debounce_ms
    When I switch to Files mode with a query
    Then the debounce still uses find_debounce_ms

  # ── Cancellation ────────────────────────────────────────────────

  Scenario: Deprecated directory search is cancelled on new keypress
    Given a directory search is running in background
    When I type an additional character
    Then the previous search thread is cancelled via AtomicBool
    And a new search starts with the updated query
    And stale results from the old search are discarded

  Scenario: Generation counter prevents stale results
    Given a search with generation=5 is in progress
    When I type a new character, incrementing generation to 6
    And the old search completes with generation=5
    Then the results with generation=5 are ignored
    And only results with generation=6 are accepted

  Scenario: Content search is not cancellable
    Given a grep search is running (blocking)
    When I type additional characters
    Then the current grep search continues until completion
    And the new query triggers a new grep after debounce
