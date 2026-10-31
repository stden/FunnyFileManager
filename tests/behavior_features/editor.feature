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
