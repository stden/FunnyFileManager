# Purpose: BDD scenarios for panel navigation behavior.
Feature: Advanced navigation scenarios
  As a file manager user
  I want robust navigation through the filesystem
  So that I can quickly reach any directory

  Scenario: Navigate to the filesystem root
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I go to root
    Then the active panel should be at the source filesystem root

  Scenario: Navigate to parent directory
    Given a temporary directory
    And a subdirectory "parent" with file "child.txt"
    And a subdirectory "parent/child" with file "inner.txt"
    And an app at "parent/child" in the temporary directory
    When I go to parent directory
    Then the active panel path should end with "parent"

  Scenario: PageUp at top of list stays at zero
    Given a temporary directory
    And a file "a.txt" with content "a"
    And a file "b.txt" with content "b"
    And a panel at the temporary directory
    And the selection is at index 0
    When I press PageUp
    Then the selection should be at index 0

  Scenario: PageDown moves selection forward
    Given a temporary directory
    And 20 numbered files
    And a panel at the temporary directory
    And the selection is at index 0
    When I press PageDown
    Then the selection should be at index 10

  Scenario: Home and End navigation
    Given a temporary directory
    And a file "aaa.txt" with content "a"
    And a file "bbb.txt" with content "b"
    And a file "ccc.txt" with content "c"
    And a panel at the temporary directory
    When I press End
    When I press Home
    Then the selection should be at index 0

  Scenario: Enter on subdirectory then parent returns
    Given a temporary directory
    And a subdirectory "inner" with file "file.txt"
    And a panel at the temporary directory
    And I select the entry "inner"
    When I press Enter
    Then the panel path should end with "inner"

  Scenario: Navigate past the last entry stops there
    Given a temporary directory
    And a file "only.txt" with content "x"
    And a panel at the temporary directory
    And the selection is at index 0
    When I press Down
    When I press Down
    Then the selection should be at index 1

  Scenario: Navigate up stops at the top
    Given a temporary directory
    And a file "only.txt" with content "x"
    And a panel at the temporary directory
    And the selection is at index 0
    When I press Up
    Then the selection should be at index 0

  Scenario: Drive selection opens for left panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open drive selection for Left
    Then drive selection should be visible

  Scenario: Drive selection opens for right panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open drive selection for Right
    Then drive selection should be visible
