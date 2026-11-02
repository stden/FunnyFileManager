# Purpose: Far Manager's editor caret and delete keys (FarEng.hlf.m4, "Cursor movement" and "Delete operations") through the real event loop. The bar's Ln, Col and Ch show where the caret is; the saved file shows what a delete removed.
Feature: Far Manager editor caret and delete keys

  Scenario: Left moves the caret one character left
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 End Left"
    Then the screen contains "Ln 1/1  Col 3  Ch 0x0063"

  Scenario: Right moves the caret one character right
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 Right"
    Then the screen contains "Ln 1/1  Col 2  Ch 0x0062"

  Scenario: Up moves the caret one line up
    Given a TUI workspace
    And the left file "notes.txt" has 3 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Down Down Up"
    Then the screen contains "Ln 2/3  Col 1  Ch 0x006c"

  Scenario: Down moves the caret one line down
    Given a TUI workspace
    And the left file "notes.txt" has 3 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Down"
    Then the screen contains "Ln 2/3  Col 1  Ch 0x006c"

  Scenario: Home moves the caret to the start of the line
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 End"
    Then the screen contains "Ln 1/1  Col 4  Ch 0x0000"
    When I press "Home"
    Then the screen contains "Ln 1/1  Col 1  Ch 0x0061"

  Scenario: End moves the caret to the end of the line
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 End"
    Then the screen contains "Ln 1/1  Col 4  Ch 0x0000"

  # A page is twenty lines, FFM's editor page; Far's help gives no page size.
  Scenario: PgDn moves the caret one page down
    Given a TUI workspace
    And the left file "notes.txt" has 50 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 PgDn"
    Then the screen contains "Ln 21/50  Col 1  Ch 0x006c"

  Scenario: PgUp moves the caret one page up
    Given a TUI workspace
    And the left file "notes.txt" has 50 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 PgDn PgDn PgUp"
    Then the screen contains "Ln 21/50  Col 1  Ch 0x006c"

  Scenario: Ctrl+Home moves the caret to the start of the file
    Given a TUI workspace
    And the left file "notes.txt" has 3 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Ctrl+End"
    Then the screen contains "Ln 3/3  Col 8  Ch 0x0000"
    When I press "Ctrl+Home"
    Then the screen contains "Ln 1/3  Col 1  Ch 0x006c"

  Scenario: Ctrl+End moves the caret to the end of the file
    Given a TUI workspace
    And the left file "notes.txt" has 3 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Ctrl+End"
    Then the screen contains "Ln 3/3  Col 8  Ch 0x0000"

  Scenario: Ctrl+PgUp moves the caret to the start of the file
    Given a TUI workspace
    And the left file "notes.txt" has 3 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Ctrl+End"
    Then the screen contains "Ln 3/3  Col 8  Ch 0x0000"
    When I press "Ctrl+PgUp"
    Then the screen contains "Ln 1/3  Col 1  Ch 0x006c"

  Scenario: Ctrl+PgDn moves the caret to the end of the file
    Given a TUI workspace
    And the left file "notes.txt" has 3 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Ctrl+PgDn"
    Then the screen contains "Ln 3/3  Col 8  Ch 0x0000"

  Scenario: Del deletes the character under the caret
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 Home Del F2"
    Then the left file "notes.txt" contains "bc"

  Scenario: BS deletes the character before the caret
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 End Backspace F2"
    Then the left file "notes.txt" contains "ab"
