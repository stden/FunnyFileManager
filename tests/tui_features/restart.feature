Feature: Remember the directories displayed in both panels
  Scenario: Navigation is saved before exit and restored after reopening
    Given a TUI workspace
    And the left file "left.txt" contains "left"
    And the right file "right.txt" contains "right"
    When I resume the TUI at 100 by 30
    And I type "cd left"
    And I press "Enter Tab"
    And I type "cd right"
    And I press "Enter"
    Then the left panel shows "left.txt"
    And the right panel shows "right.txt"
    And the saved panel directories are "left" and "right"
    When I press "F10 Enter"
    Then the TUI exits successfully
    When I resume the TUI at 100 by 30
    Then the left panel shows "left.txt"
    And the right panel shows "right.txt"
    When I press "Ctrl+U"
    Then the saved panel directories are "right" and "left"
    When I press "F10 Enter"
    Then the TUI exits successfully
    When I resume the TUI at 100 by 30
    Then the left panel shows "right.txt"
    And the right panel shows "left.txt"
    When I press "F10 Enter"
    Then the TUI exits successfully

  Scenario: A removed saved directory leaves the other panel intact
    Given a TUI workspace
    And the left file "left.txt" contains "left"
    And the right file "right.txt" contains "right"
    When I resume the TUI at 100 by 30
    And I type "cd left"
    And I press "Enter Tab"
    And I type "cd right"
    And I press "Enter F10 Enter"
    Then the TUI exits successfully
    When the left workspace directory is removed
    And I resume the TUI at 100 by 30
    Then the left panel shows "right"
    And the right panel shows "right.txt"
    When I press "F10 Enter"
    Then the TUI exits successfully
