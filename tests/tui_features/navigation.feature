Feature: Keyboard navigation through the rendered TUI
  Scenario: Help consumes panel shortcuts and returns to the panels
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello from the viewer"
    When I open the TUI at 100 by 30
    Then the left panel shows "notes.txt"
    When I press "Down F1"
    Then the screen contains "Help"
    When I press "F8"
    Then the left file "notes.txt" contains "Hello from the viewer"
    When I press "Esc F3"
    Then the screen contains "Hello from the viewer"
    When I press "Esc F10"
    Then the screen contains "quit"
    When I press "Enter"
    Then the TUI exits successfully

  Scenario: Panel switching and terminal resizing keep files visible
    Given a TUI workspace
    And the left file "left.txt" contains "left"
    And the right file "right.txt" contains "right"
    When I open the TUI at 100 by 30
    And I press "Tab Down F3"
    Then the screen contains "right"
    When I press "Esc"
    And I resize the TUI to 50 by 12
    Then the screen size is 50 by 12
    And the left panel shows "left.txt"
    And the right panel shows "right.txt"
    When I press "F10 Enter"
    Then the TUI exits successfully
