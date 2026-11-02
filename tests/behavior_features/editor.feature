Feature: The editor opens files, accepts input and closes cleanly
  Background:
    Given an isolated acceptance workspace

  Scenario: F4 opens the editor for the selected file
    Given a file named "notes.txt" containing "hello"
    When I dispatch the keys "F4"
    Then the editor is open
    And the editor buffer equals "hello"

  Scenario: Esc closes an unmodified editor immediately
    Given a file named "clean.txt" containing "original"
    When I dispatch the keys "F4 Esc"
    Then the overlay is closed

  Scenario: Typing inserts characters at the cursor position
    Given a file named "code.txt" containing "world"
    When I dispatch the keys "F4 Home"
    And I type the text "hello "
    Then the editor buffer equals "hello world"

  Scenario: Esc on a modified buffer prompts whether to save
    Given a file named "draft.txt" containing "original"
    When I dispatch the keys "F4"
    And I type the text "x"
    And I dispatch the keys "Esc"
    Then the editor asks whether to save changes
    And the rendered screen contains "File has been modified. Save?"

  Scenario: Clicking in the editor body moves the caret while clicking the top bar does not
    Given a multi-line file "click.txt" with lines:
      | alpha   |
      | bravo   |
      | charlie |
    When I dispatch the keys "F4"
    And I left-click at column 3 and row 2
    Then the editor caret sits at line 1 and column 3
    When I left-click at column 1 and row 0
    Then the editor caret sits at line 1 and column 3
    When I type the text "!"
    Then editor line 1 equals "bra!vo"

  Scenario: Far keys F4 and Ctrl+PgDn and Ctrl+PgUp navigate and close the editor
    Given a multi-line file "far_keys.txt" with lines:
      | first  |
      | middle |
      | final  |
    When I dispatch the keys "F4 Ctrl+PgDn"
    Then the editor caret sits at line 2 and column 5
    When I dispatch the keys "Ctrl+PgUp"
    Then the editor caret sits at line 0 and column 0
    When I dispatch the keys "F4"
    Then the overlay is closed

  Scenario: F8 toggles between Windows-1251 and OEM-866 unless unsaved edits exist
    Given a Windows-1251 file named "legacy.txt" encoding "Мир"
    When I dispatch the keys "F4"
    Then the editor buffer equals "Мир"
    When I dispatch the keys "F8"
    Then the rendered screen contains "Reloaded as OEM-866"
    When I dispatch the keys "F8"
    Then the rendered screen contains "Reloaded as Windows-1251"
    And the editor buffer equals "Мир"
    When I type the text "!"
    And I dispatch the keys "F8"
    Then the rendered screen contains "Save or discard changes before reloading the file"

  Scenario: Typst documents highlight directives strings and comments
    Given a multi-line file "paper.typ" with lines:
      | #set page(width: 10pt) |
      | let title = "Hello"    |
      | // trailing note       |
    When I dispatch the keys "F4"
    Then the editor screen paints Typst directive string and comment colors
