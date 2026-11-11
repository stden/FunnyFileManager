# Purpose: BDD scenarios for sorting behavior.
Feature: Advanced sorting scenarios
  As a file manager user
  I want sorting to work correctly across all modes
  So that I can find files quickly

  Scenario: Sort by name is the default
    Given a temporary directory
    And a file "c.txt" with content "c"
    And a file "a.txt" with content "a"
    And a file "b.txt" with content "b"
    When I create a panel with sort mode Name
    Then entry 0 should be ".."

  Scenario: Sort by extension groups files by type
    Given a temporary directory
    And a file "data.csv" with content "1"
    And a file "notes.txt" with content "2"
    And a file "image.png" with content "3"
    When I create a panel with sort mode Extension
    Then the first extension should come before the last alphabetically

  Scenario: Cycle sort mode three times returns to Name
    Given a temporary directory
    And a file "test.txt" with content "t"
    And a panel at the temporary directory
    When I cycle sort mode 3 times
    Then the sort mode should be back to Name

  Scenario: Sort command via command line
    Given a source directory
    And a file "z.txt" with content "z" in the source
    And a file "a.txt" with content "a" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "sort name"
    Then the sort mode should be Name
    When I execute command "sort ext"
    Then the sort mode should be Extension
    When I execute command "sort size"
    Then the sort mode should be Size
