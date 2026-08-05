Feature: Enter Key Behavior
  As a user
  I want the Enter key to navigate to the selected item
  So that I can drill into directories or files

  Background:
    Given the TUI is open

  # ── Enter in List ───────────────────────────────────────────────

  Scenario: Enter on a directory navigates into it
    Given the current mode is "Directories"
    And a directory "projects" is selected
    When I press Enter
    Then current_dir changes to "projects"
    And the search bar is cleared
    And the preview resets
    And the find cache is invalidated
    And the list refreshes with the new directory's children

  Scenario: Enter on a file in File mode navigates to parent
    Given the current mode is "Files"
    And a file "src/main.rs" is selected
    When I press Enter
    Then current_dir changes to "src" (the file's parent)
    And the mode switches to "Directories"
    And the search bar is cleared

  Scenario: Enter on a grep result navigates to parent directory
    Given the current mode is "Content"
    And a grep result "src/lib.rs:15" is selected
    When I press Enter
    Then current_dir changes to "src" (the file's parent)
    And the mode switches to "Directories"
    And the search bar is cleared

  Scenario: Enter on nothing (no selection) does nothing
    Given no item is selected in the list
    When I press Enter
    Then nothing changes

  # ── Enter in Preview ────────────────────────────────────────────

  Scenario: Enter on a directory entry in preview navigates into it
    Given focus is on the preview zone
    And a directory entry "src" is selected in the preview tree
    When I press Enter
    Then current_dir changes to "src"
    And focus returns to the list
    And the list refreshes

  Scenario: Enter on a file entry in preview does nothing
    Given focus is on the preview zone
    And a file entry "main.rs" is selected in the preview tree
    When I press Enter
    Then nothing changes (files in preview are not navigable)
