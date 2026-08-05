Feature: Configuration
  As a user
  I want to customize cdx behavior through a config file
  So that the tool works the way I prefer

  Background:
    Given the config file is at ~/.config/cdx/config.toml

  # ── Config loading ──────────────────────────────────────────────

  Scenario: Config is loaded on startup
    Given ~/.config/cdx/config.toml exists
    When cdx starts
    Then the config is parsed and applied

  Scenario: Missing config creates default
    Given ~/.config/cdx/config.toml does not exist
    When cdx starts
    Then config.default.toml is copied to ~/.config/cdx/config.toml
    And default values are used

  Scenario: Invalid config falls back to defaults
    Given ~/.config/cdx/config.toml has syntax errors
    When cdx starts
    Then an error message is printed to stderr
    And default values are used

  # ── Debounce values ─────────────────────────────────────────────

  Scenario: find_debounce_ms controls directory/file search debounce
    Given find_debounce_ms is set to 100
    When I type in the search bar in Directories or Files mode
    Then the search is delayed by 100ms after the last keypress

  Scenario: grep_debounce_ms controls content search debounce
    Given grep_debounce_ms is set to 200
    When I type in the search bar in Content mode
    Then the grep is delayed by 200ms after the last keypress

  # ── Keybindings ─────────────────────────────────────────────────

  Scenario: Keybindings are configurable
    Given the config has:
      | action           | key     |
      | quit             | ctrl+c  |
      | toggle_dotfiles  | ctrl+h  |
      | switch_mode      | tab     |
      | open_explorer    | ctrl+o  |
      | open_settings    | ctrl+e  |
    When cdx starts
    Then pressing the configured key triggers the corresponding action

  Scenario: Custom keybinding format is parsed correctly
    Given the config has quit = "ctrl+shift+q"
    When the keybinding is parsed
    Then the modifier includes CONTROL and SHIFT
    And the key code is 'q'

  # ── Tool selector ───────────────────────────────────────────────

  Scenario: Tool selector entries are configurable
    Given the config has:
      | name    | command  |
      | yazi    | yazi     |
      | nvim    | nvim     |
      | opencode| opencode |
      | lazygit | lazygit  |
    When the tool selector opens
    Then all 4 tools are shown in the configured order

  Scenario: Custom tools can be added
    Given the config adds a tool: name="code", command="code"
    When the tool selector opens
    Then "code" appears in the list

  # ── Search configuration ────────────────────────────────────────

  Scenario: grep_max_depth limits rg depth
    Given grep_max_depth is set to 3
    When a grep search runs
    Then `--max-depth 3` is passed to rg

  Scenario: max_secondary_depth limits file/dir walk depth
    Given max_secondary_depth is set to 8
    When file or directory search runs
    Then WalkBuilder uses max_depth(8)

  Scenario: exclude_dirs filters directories from all modes
    Given exclude_dirs contains ["node_modules", ".git"]
    When any search mode runs
    Then entries matching those names are excluded

  Scenario: zoxide_limit controls max zoxide entries shown
    Given zoxide_limit is set to 3
    When the list loads for the first time
    Then at most 3 zoxide entries are shown
