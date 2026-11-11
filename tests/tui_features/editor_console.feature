Feature: Editor and console workflows through keyboard input
  Scenario: Ctrl+C copies selected Unicode text and keeps the editor open
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello мир🙂"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End Ctrl+C"
    Then the clipboard contains "Hello мир🙂"
    And the screen contains "Selection copied"
    And the left file "notes.txt" contains "Hello мир🙂"
    When I press "Esc F10 Enter"
    Then the TUI exits successfully

  Scenario: Ctrl+V pastes copied Unicode text and saves it
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello мир🙂"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End Ctrl+C End Ctrl+V F2 Esc"
    Then the left file "notes.txt" contains "Hello мир🙂Hello мир🙂"
    When I press "F10 Enter"
    Then the TUI exits successfully

  Scenario: A terminal paste replaces the selected text
    Given a TUI workspace
    And the left file "notes.txt" contains "old text"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+End"
    And the terminal pastes "New мир🙂"
    Then the screen contains "New мир🙂"
    When I press "F2 Esc"
    Then the left file "notes.txt" contains "New мир🙂"
    When I press "F10 Enter"
    Then the TUI exits successfully

  Scenario: Edit and save a Unicode text file
    Given a TUI workspace
    And the left file "notes.txt" contains "Hello"
    When I open the TUI at 100 by 30
    And I press "Down F4 End"
    And I type " мир"
    Then the screen contains "Hello мир"
    When I press "F2 Esc"
    Then the left file "notes.txt" contains "Hello мир"
    And the left panel shows "notes.txt"
    When I press "F10 Enter"
    Then the TUI exits successfully

  Scenario: Execute a shell command in the console and return to panels
    Given a TUI workspace
    And the left file "notes.txt" contains "untouched"
    When I open the TUI at 100 by 30
    And I press "Ctrl+O"
    And I type "echo ui-ready"
    And I press "Enter"
    Then the screen contains "ui-ready"
    When I press "Ctrl+O"
    Then the left panel shows "notes.txt"
    When I press "F10 Enter"
    Then the TUI exits successfully
