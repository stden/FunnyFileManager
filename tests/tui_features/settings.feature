# Purpose: Verify that Save setup keeps the settings in settings.yaml, migrates
# the legacy settings.json without touching it, keeps what the user changed in
# the file while FFM ran, and never overwrites a settings file that does not
# parse.
Feature: Settings file
  Scenario: Save setup migrates settings.json into settings.yaml and keeps the language servers
    Given a TUI workspace
    And the workspace settings file "settings.json" holds:
      """
      {"Editor": "8", "lsp": {"python": {"enabled": false}}}
      """
    When I open the TUI on the workspace settings at 160 by 30
    And I press "Shift+F9"
    # The path gives way from its head, never the file name: on a Windows
    # runner the message, temp path and all, is wider than a panel's row.
    Then one screen line contains "Setup saved to" and then "settings.yaml"
    And the workspace settings file "settings.yaml" contains:
      """
      lsp:
        python:
          enabled: false
      options:
        Editor: '8'
      """
    And the workspace settings file "settings.json" still holds:
      """
      {"Editor": "8", "lsp": {"python": {"enabled": false}}}
      """

  Scenario: Save setup keeps a settings file edited by hand while FFM runs
    Given a TUI workspace
    And the workspace settings file "settings.yaml" holds:
      """
      options:
        Editor: '8'
      """
    When I open the TUI on the workspace settings at 160 by 30
    And the workspace settings file "settings.yaml" is edited to hold:
      """
      # tab size for the built-in editor
      options:
        Editor: 2
      """
    And I press "Shift+F9"
    Then the screen contains "Setup saved to"
    And the workspace settings file "settings.yaml" still holds:
      """
      # tab size for the built-in editor
      options:
        Editor: 2
      """

  Scenario: A settings file that does not parse is reported and never overwritten
    Given a TUI workspace
    And the workspace settings file "settings.yaml" holds:
      """
      options:
        Editor: '8'
        Editor: '9'
      """
    When I open the TUI on the workspace settings at 160 by 30
    Then the screen contains "settings.yaml does not parse"
    When I press "Shift+F9"
    Then the screen contains "Not saved: settings.yaml does not parse"
    And the workspace settings file "settings.yaml" still holds:
      """
      options:
        Editor: '8'
        Editor: '9'
      """
