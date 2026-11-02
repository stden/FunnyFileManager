# Purpose: Far Manager's editor selection, clipboard and file keys (FarEng.hlf.m4, "Block operations" and "Other operations") through the real event loop. The selection is the text painted in the selection colour, the clipboard is the clipboard backend, and the file is read from disk.
Feature: Far Manager editor selection, clipboard and file keys

  Scenario: Shift+Left selects one character to the left
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello"
    When I open the TUI at 100 by 30
    And I press "Down F4 End Shift+Left Shift+Left"
    Then the screen highlights "lo"

  Scenario: Shift+Right selects one character to the right
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+Right Shift+Right"
    Then the screen highlights "He"

  # A selection across a line break paints one more cell after that line's text,
  # so the vertical keys are checked by what typing replaces in the file.
  Scenario: Shift+Up selects the line above
    Given a TUI workspace
    And the left file "notes.txt" holds:
      """
      abc
      def
      """
    When I open the TUI at 100 by 30
    And I press "Down F4 Down Shift+Up"
    And I type "X"
    And I press "F2"
    Then the left file "notes.txt" contains "Xdef"

  Scenario: Shift+Down selects the line below
    Given a TUI workspace
    And the left file "notes.txt" holds:
      """
      abc
      def
      """
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+Down"
    And I type "X"
    And I press "F2"
    Then the left file "notes.txt" contains "Xdef"

  Scenario: Shift+Home selects to the start of the line
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello"
    When I open the TUI at 100 by 30
    And I press "Down F4 End Shift+Home"
    Then the screen highlights "Hello"

  Scenario: Shift+End selects to the end of the line
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End"
    Then the screen highlights "Hello"

  # A page is twenty lines, FFM's editor page: typing over the selection leaves
  # 30 of 50 lines. Far's help gives no page size.
  Scenario: Shift+PgDn selects one page down
    Given a TUI workspace
    And the left file "notes.txt" has 50 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+PgDn"
    And I type "X"
    Then the screen contains "Ln 1/30  Col 2  Ch 0x006c"

  Scenario: Shift+PgUp selects one page up
    Given a TUI workspace
    And the left file "notes.txt" has 50 numbered lines
    When I open the TUI at 100 by 30
    And I press "Down F4 PgDn PgDn Shift+PgUp"
    And I type "X"
    Then the screen contains "Ln 21/30  Col 2  Ch 0x006c"

  Scenario: Ctrl+Shift+Home selects to the start of the file
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 End Ctrl+Shift+Home"
    Then the screen highlights "abc"

  Scenario: Ctrl+Shift+End selects to the end of the file
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 Ctrl+Shift+End"
    Then the screen highlights "abc"

  Scenario: Ctrl+A selects the whole text
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 Ctrl+A"
    Then the screen highlights "abc"

  Scenario: Ctrl+C copies the selection
    Given a TUI workspace
    And the left file "notes.txt" contains "copy me"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End Ctrl+C"
    Then the clipboard contains "copy me"
    And the screen contains "Selection copied"

  Scenario: Ctrl+Ins copies the selection
    Given a TUI workspace
    And the left file "notes.txt" contains "insert me"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End Ctrl+Ins"
    Then the clipboard contains "insert me"
    And the screen contains "Selection copied"

  Scenario: Ctrl+V pastes the clipboard at the caret
    Given a TUI workspace
    And the left file "notes.txt" contains "ab"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End Ctrl+C End Ctrl+V F2"
    Then the left file "notes.txt" contains "abab"

  Scenario: Shift+Ins pastes the clipboard at the caret
    Given a TUI workspace
    And the left file "notes.txt" contains "ab"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End Ctrl+C End Shift+Ins F2"
    Then the left file "notes.txt" contains "abab"

  Scenario: Ctrl+F inserts the full file name at the caret
    Given a TUI workspace
    And the left file "notes.txt" contains "name: "
    When I open the TUI at 100 by 30
    And I press "Down F4 End Ctrl+F F2"
    Then the left file "notes.txt" is "name: " followed by its full path

  Scenario: F2 saves the file and keeps the editor open
    Given a TUI workspace
    And the left file "notes.txt" contains "abc"
    When I open the TUI at 100 by 30
    And I press "Down F4 End"
    And I type "x"
    And I press "F2"
    Then the left file "notes.txt" contains "abcx"
    And the screen contains "Ln 1/1  Col 5  Ch 0x0000"

