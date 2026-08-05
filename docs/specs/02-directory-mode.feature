Feature: Directory Search Mode
  As a user
  I want to search for directories using different strategies depending on whether I have a query
  So that I can quickly navigate to any directory

  Background:
    Given the TUI is open
    And the current mode is "Directories"

  # ── Sin query ───────────────────────────────────────────────────

  Scenario: First open shows direct children + zoxide dirs
    Given the current directory is "~/dev"
    And the search bar is empty
    When the list loads for the first time
    Then the list shows immediate subdirectories of "~/dev"
    And the list includes up to 5 zoxide directories (most used)
    And zoxide items are marked with a star icon (★)
    And zoxide items are displayed with yellow bold style

  Scenario: Zoxide directories are merged only on first load
    Given the search bar is empty
    And zoxide items are currently visible
    When I type any character into the search bar
    Then the zoxide items disappear from the list
    And only matching directories are shown

  Scenario: Zoxide directories excluded if already in cwd children
    Given "~/dev/projects" is a direct child of "~/dev"
    And "~/dev/projects" is also in the zoxide list
    When the list loads
    Then "~/dev/projects" appears only once (as a direct child)
    And zoxide does not duplicate it

  Scenario: Zoxide directories exclude current directory
    Given the current directory is "~/dev"
    And "~/dev" is in the zoxide list
    When the list loads
    Then "~/dev" does not appear as a zoxide item

  Scenario: Directories use read_dir (1 level deep)
    Given the current directory has nested subdirectories
    When the list loads
    Then only immediate children (1 level) are shown
    And deeper subdirectories are not included

  Scenario: exclude_dirs filter applies
    Given "node_modules" is in exclude_dirs
    And a directory "node_modules" exists in cwd
    When the list loads
    Then "node_modules" is not shown

  Scenario: show_dotfiles filter applies
    Given show_dotfiles is false
    And a directory ".config" exists in cwd
    When the list loads
    Then ".config" is not shown

  Scenario: show_winhidden filter applies on Windows
    Given show_winhidden is false
    And a directory "AppData" exists in cwd
    When the list loads
    Then "AppData" is not shown

  Scenario: .ignore file does NOT affect directory listing
    Given a ".ignore" file exists in cwd containing "Dropbox/"
    And a directory "Dropbox" exists in cwd
    When the list loads
    Then "Dropbox" IS shown (read_dir ignores .ignore)

  # ── Con query ───────────────────────────────────────────────────

  Scenario: Query triggers async recursive directory search
    Given I type "proj" in the search bar
    When the debounce elapses
    Then a background thread spawns recursive_dir_search
    And the search uses WalkBuilder with max_secondary_depth (5)

  Scenario: Recursive search applies exclude_dirs
    Given I type "src" in the search bar
    And "target" is in exclude_dirs
    When the recursive search runs
    Then directories named "target" are excluded from results

  Scenario: Recursive search applies show_dotfiles
    Given show_dotfiles is true
    And I type "conf" in the search bar
    When the recursive search runs
    Then dotfiles directories like ".config" ARE included

  Scenario: Recursive search applies .ignore rules
    Given a ".ignore" file exists with rules
    And I type "proj" in the search bar
    When the recursive search runs
    Then entries matching .ignore rules are excluded

  Scenario: Results are fuzzy matched with nucleo
    Given the recursive search has returned 200+ directories
    When the results are scored with the query "proj"
    Then only the top 200 matches by nucleo score are shown
    And results are sorted by relevance

  Scenario: Cancellation stops recursive search mid-walk
    Given a recursive search is running on a large tree
    When I type a new character before the search completes
    Then the AtomicBool cancel flag is set
    And the WalkBuilder loop breaks early
    And stale results are not displayed
