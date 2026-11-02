Feature: Scoped PATH editor
  Scenario: Invalid directories cannot be saved and drafts can be discarded
    Given a TUI workspace
    When I open the TUI at 120 by 35
    And I press "F9 Right Right Down Down Down Enter"
    Then the screen contains "PATH - current user"
    When I press "F7"
    And I type the missing workspace directory
    And I press "Enter"
    Then the screen contains "MISSING"
    When I press "F2 Enter"
    Then the screen contains "Fix or remove invalid paths"
    When I press "Esc"
    Then the screen contains "Discard unsaved PATH changes"
    When I press "Enter F9 Right Right Down Down Down Down Enter"
    Then the screen contains "PATH - all users"
    When I press "Esc"
    Then the screen contains "Commands"
