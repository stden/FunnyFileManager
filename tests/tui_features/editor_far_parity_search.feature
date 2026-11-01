# Purpose: Far Manager's editor search and close keys (FarEng.hlf.m4, "Other operations") through the real event loop: the bar shows where the search left the caret, and the close keys ask before they discard a change.
Feature: Far Manager editor search and close keys

  Scenario: F7 finds the text and moves the caret to it
    Given a TUI workspace
    And the left file "notes.txt" holds:
      """
      alpha
      beta gamma
      """
    When I open the TUI at 100 by 30
    And I press "Down F4 F7"
    And I type "beta"
    And I press "Enter"
    Then the screen contains "Ln 2/2  Col 1  Ch 0x0062"

  Scenario: Shift+F7 finds the next match
    Given a TUI workspace
    And the left file "notes.txt" holds:
      """
      x beta
      beta y
      """
    When I open the TUI at 100 by 30
    And I press "Down F4 F7"
    And I type "beta"
    And I press "Enter Shift+F7"
    Then the screen contains "Ln 2/2  Col 1  Ch 0x0062"

  Scenario: F10 closes the editor and shows the panels
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 F10"
    Then the screen does not contain "Ln 1/1"
    And the left panel shows "notes.txt"

  Scenario: Esc closes the editor and shows the panels
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 Esc"
    Then the screen does not contain "Ln 1/1"
    And the left panel shows "notes.txt"

  Scenario: F4 closes the editor and shows the panels
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 F4"
    Then the screen does not contain "Ln 1/1"
    And the left panel shows "notes.txt"

  Scenario: F4 asks before closing a changed file, and n discards the change
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4"
    And I type "x"
    And I press "F4"
    Then the screen contains "File has been modified. Save?"
    When I press "n"
    Then the left file "notes.txt" contains "abc"
    And the left panel shows "notes.txt"
