Feature: Escape Key Behavior
  As a user
  I want the Esc key to navigate up or exit
  So that I can move through the directory tree or leave the TUI

  Background:
    Given the TUI is open

  # ── Single Esc ──────────────────────────────────────────────────

  Scenario: Esc in list navigates to parent directory
    Given the current directory is "~/dev/projects"
    And focus is on the list
    When I press Esc once
    Then current_dir changes to "~/dev"
    And the list refreshes with "~/dev" contents
    And the current query (if any) is re-applied

  Scenario: Esc re-executes search if query is active
    Given the current directory is "~/dev/projects"
    And the search bar contains "myapp"
    When I press Esc once
    Then current_dir changes to "~/dev"
    And the search "myapp" is re-executed in "~/dev"

  Scenario: Esc in preview returns focus to list
    Given focus is on the preview zone
    When I press Esc
    Then focus returns to the list zone
    And the current directory does not change

  Scenario: Esc at root does not exit
    Given the current directory is "/" (root)
    When I press Esc once
    Then nothing happens (already at filesystem root)

  # ── Double Esc (rapid) ──────────────────────────────────────────

  Scenario: Double Esc within 300ms navigates to home
    Given the current directory is "~/dev/projects/deep/nested"
    When I press Esc twice within 300ms
    Then current_dir changes to "~" (home directory)
    And the list refreshes with home contents

  Scenario: Esc with delay > 300ms is treated as single Esc
    Given the current directory is "~/dev/projects"
    When I press Esc
    And I wait 400ms
    And I press Esc again
    Then the first Esc navigates to "~/dev"
    And the second Esc navigates to "~"

  Scenario: Double Esc from any depth goes to home
    Given the current directory is anywhere except home
    When I press Esc twice rapidly
    Then current_dir is always "~" (home)
