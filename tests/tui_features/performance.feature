Feature: Action waiting statistics
  Scenario: Opening the report through the command line
    Given a TUI workspace
    When I open the TUI at 120 by 30
    And I press "Tab Tab"
    And I type "stats"
    And I press "Enter"
    Then the screen contains "Action statistics"
    And the screen contains "Total ms"
    And the screen contains "Mean ms"
    And the screen contains "SwitchPanel"
