Feature: Panel actions preserve file identity and keyboard intent
  Background:
    Given an isolated acceptance workspace

  Scenario: Enter opens a document by association with its literal path
    Given the selected document is named "report & notes.txt"
    When I dispatch the keys "Enter"
    Then the system association opener receives the literal document "report & notes.txt"

  Scenario: F3 still requests the internal viewer explicitly
    Given the selected document is named "notes.txt"
    When I dispatch the keys "F3"
    Then the rendered screen contains "fixture document body"

  Scenario: Refresh recovers when the current directory was removed
    Given the left panel is inside a directory that disappears
    When I dispatch the keys "Ctrl+R"
    Then refresh restores the nearest existing left directory

  Scenario Outline: An Alt heading shortcut changes the open menu without running a row
    When I dispatch the keys "F9 Down <key>"
    Then the active menu heading is <heading>
    Examples:
      | key   | heading  |
      | Alt+L | Left     |
      | Alt+F | Files    |
      | Alt+C | Commands |
      | Alt+O | Options  |
      | Alt+R | Right    |
