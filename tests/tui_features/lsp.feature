# Purpose: Verify the Language servers dialog through the real event loop: opening it, editing, completing, saving and refusing.
Feature: Language servers dialog
  Background:
    Given a TUI workspace
    And the fake machine has the program "fixture-server"
    And the fake machine has the program "python3"
    And the workspace settings file "settings.yaml" holds:
      """
      options:
        System: '5000'
      unknown:
        keep: 1
      lsp:
        fixture:
          command: fixture-server
          args: [--stdio]
          extensions: [fx]
          root_markers: [.git]
          install: null
      """

  Scenario: The Commands menu opens the dialog
    When I open the language servers workspace
    And I press "F9 Right Right Down Enter"
    Then the screen contains "Language servers"
    And the screen shows the settings file path
    And the screen contains "fixture"
    And the screen contains "2Save"
    When I press "Down Down"
    Then the screen contains "found configured executable"

  Scenario: The lsp command opens it
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter"
    Then the screen contains "Language servers"

  Scenario: Toggle and save keep foreign keys
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter Down Down Space F2"
    Then the workspace settings file "settings.yaml" value "lsp.fixture.enabled" is "false"
    And the workspace settings file "settings.yaml" value "lsp.fixture.root_markers" is "[.git]"
    And the workspace settings file "settings.yaml" value "unknown.keep" is "1"
    And the workspace settings file "settings.yaml" value "options.System" is "5000"

  Scenario: Completing a command
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter Down Down Down Down Down Down Down Down F4"
    And I press "Backspace Backspace Backspace Backspace Backspace"
    And I type "rust-an"
    Then the screen contains "rust-analyzer"
    And the screen contains "catalog"
    When I press "Down Enter Enter"
    Then the screen contains "recipe"
    When I press "F2"
    Then the workspace settings file "settings.yaml" value "lsp.python.command" is "rust-analyzer"

  Scenario: A duplicate extension blocks saving
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter Down Down F4 Tab Tab"
    And I type " py"
    And I press "Enter F2"
    Then the screen contains "claimed by"
    And the workspace settings file "settings.yaml" still holds:
      """
      options:
        System: '5000'
      unknown:
        keep: 1
      lsp:
        fixture:
          command: fixture-server
          args: [--stdio]
          extensions: [fx]
          root_markers: [.git]
          install: null
      """

  Scenario: Discarding
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter Space Esc"
    Then the screen contains "Save?"
    When I press "n"
    Then the workspace settings file "settings.yaml" still holds:
      """
      options:
        System: '5000'
      unknown:
        keep: 1
      lsp:
        fixture:
          command: fixture-server
          args: [--stdio]
          extensions: [fx]
          root_markers: [.git]
          install: null
      """
    And the screen does not contain "Language servers"

  Scenario: A file that does not parse is read-only
    Given the workspace settings file "settings.yaml" holds:
      """
      {broken
      """
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter"
    Then the screen contains "read only"
    And the screen contains "does not parse"
    And the screen contains "line 2"
    When I press "F2"
    Then the workspace settings file "settings.yaml" still holds:
      """
      {broken
      """

  Scenario: Shift+F9 keeps the LSP edit
    When I open the language servers workspace
    And I type "lsp"
    And I press "Enter Down Down Space F2 Esc Shift+F9"
    Then the workspace settings file "settings.yaml" value "lsp.fixture.enabled" is "false"
    And the workspace settings file "settings.yaml" value "options.System" is "5000"
