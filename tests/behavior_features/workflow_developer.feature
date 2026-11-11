Feature: Developer workflow — view and edit source files

  Background:
    Given an isolated acceptance workspace
    And the left panel has 20 source files with .rs extension

  Scenario: Developer uses quick-search to jump to a file starting with "m" and views it
    When I dispatch the keys "Alt+m"
    Then the left panel cursor is on a file starting with "m"
    When I dispatch the keys "F3"
    Then the overlay is the viewer
    When I dispatch the keys "Esc"
    Then the overlay is closed

  Scenario: Developer opens and closes the viewer with F3 toggle
    When I dispatch the keys "F3"
    Then the overlay is the viewer
    When I dispatch the keys "F3"
    Then the overlay is closed

  Scenario: Developer opens the editor, types text, then cancels
    When I dispatch the keys "F4"
    Then the editor is open
    When I type the text "hello world"
    And I dispatch the keys "Esc"
    Then the editor asks whether to save changes
