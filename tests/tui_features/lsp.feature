# Purpose: Verify LSP inventory discovery from the F9 menu and command line.
Feature: Language server inventory
  Scenario: F9 Commands shows server installation and launch parameters
    Given a TUI workspace
    When I open the LSP inventory fixture
    And I press "F9 Right Right Down Enter"
    Then the screen contains "Language servers"
    And the screen contains "Configured servers"
    And the screen contains "Launch argument: --stdio"
    When I press "Esc"
    And I type "lsp"
    And I press "Enter"
    Then the screen contains "Language servers"
