Feature: Ctrl+ Keybindings
  As a user
  I want keyboard shortcuts for common actions
  So that I can work efficiently without leaving the keyboard

  Background:
    Given the TUI is open

  # ── Ctrl+O: Tool Selector ──────────────────────────────────────

  Scenario: Ctrl+O opens tool selector popup
    Given a directory is selected in the list
    When I press Ctrl+O
    Then a popup appears with the tool selector
    And the first tool (yazi) is highlighted

  Scenario: Tool selector shows configurable tools
    Given the config has tool_selector: [yazi, nvim, opencode, lazygit]
    When the tool selector opens
    Then 4 tools are listed in order

  Scenario: Navigate tool selector with arrows or j/k
    Given the tool selector is open
    When I press Down or j
    Then the highlight moves to the next tool
    When I press Up or k
    Then the highlight moves to the previous tool

  Scenario: Confirm tool selection with Enter
    Given the tool selector is open
    And "nvim" is highlighted
    When I press Enter
    Then the popup closes
    And nvim is spawned inline with the selected directory
    And the TUI pauses (not closes)
    And after nvim exits, the TUI resumes

  Scenario: Cancel tool selector with Esc or q
    Given the tool selector is open
    When I press Esc or q
    Then the popup closes
    And no tool is spawned

  Scenario: Ctrl+O does nothing if no directory is selected
    Given no directory is selected (only files or empty)
    When I press Ctrl+O
    Then nothing happens

  # ── Ctrl+E: Open with nvim ─────────────────────────────────────

  Scenario: Ctrl+E opens config with nvim
    Given I press Ctrl+E
    Then nvim is spawned inline with ~/.config/cdx/config.toml
    And the TUI pauses
    And after nvim exits, the TUI resumes

  Scenario: Ctrl+E is inline (pauses TUI, doesn't close)
    Given the TUI is running
    When Ctrl+E triggers nvim
    Then ratatui is restored after nvim exits
    And the TUI continues from where it left off

  # ── Ctrl+C: Quit ───────────────────────────────────────────────

  Scenario: Ctrl+C exits the TUI
    Given the TUI is running
    When I press Ctrl+C
    Then the TUI closes
    And the current directory is printed to stdout
    And the current directory is written to the result file

  Scenario: Ctrl+C outputs cwd regardless of selection
    Given the TUI is running
    And I have navigated to "~/dev/projects"
    When I press Ctrl+C
    Then "~/dev/projects" is printed to stdout

  # ── Ctrl+H: Toggle dotfiles ────────────────────────────────────

  Scenario: Ctrl+H toggles dotfiles visibility
    Given show_dotfiles is false
    When I press Ctrl+H
    Then show_dotfiles becomes true
    And the find cache is invalidated
    And the list refreshes showing dotfiles

  Scenario: Ctrl+H again hides dotfiles
    Given show_dotfiles is true
    When I press Ctrl+H
    Then show_dotfiles becomes false
    And dotfiles are hidden from the list

  # ── Ctrl+W: Toggle WinHidden (Windows) ─────────────────────────

  Scenario: Ctrl+W toggles WinHidden visibility
    Given show_winhidden is false (Windows)
    When I press Ctrl+W
    Then show_winhidden becomes true
    And the list refreshes showing Windows hidden dirs

  Scenario: Ctrl+W auto-disabled in grep mode
    Given show_winhidden is true
    And the current mode is "Content"
    When I press Ctrl+W
    Then show_winhidden toggles
    And if now true, a warning is printed
