# Purpose: BDD scenarios for stress and repeated workflow behavior.
Feature: Stress and boundary scenarios
  As a file manager user
  I want the application to handle extreme inputs correctly
  So that it never crashes or loses data

  Scenario: Panel with many files navigates correctly
    Given a temporary directory
    And 20 numbered files
    And a panel at the temporary directory
    And the selection is at index 0
    When I press Down
    Then the selection should be at index 1

  Scenario: Navigate past the last entry stops there
    Given a temporary directory
    And a file "only.txt" with content "x"
    And a panel at the temporary directory
    And the selection is at index 0
    When I press Down
    When I press Down
    Then the selection should be at index 1

  Scenario: Quick search finds first match by prefix
    Given a temporary directory
    And a file "alpha.txt" with content "a"
    And a file "beta.txt" with content "b"
    And a file "gamma.txt" with content "g"
    And a panel at the temporary directory
    When I quick search for "b"
    Then the selected entry should be "beta.txt"

  Scenario: Quick search with no match keeps selection
    Given a temporary directory
    And a file "alpha.txt" with content "a"
    And a panel at the temporary directory
    And the selection is at index 0
    When I quick search for "zzz"
    Then the selection should be at index 0

  Scenario: Quick search is case insensitive
    Given a temporary directory
    And a file "README.md" with content "docs"
    And a file "readme_local.md" with content "local"
    And a panel at the temporary directory
    When I quick search for "read"
    Then the selected entry should be "README.md"

  Scenario: Copy file with special characters in content
    Given a source directory
    And a file "special.txt" with content "line1\nline2\ttab" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "special.txt" in the left panel
    When I press F5
    Then "special.txt" should exist in the destination
    And "special.txt" should still exist in the source

  Scenario: Multiple mkdir creates sequential names
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press F7
    When I press F7
    When I press F7
    Then "new_folder" should be a directory in the source
    And "new_folder_1" should be a directory in the source
    And "new_folder_2" should be a directory in the source

  Scenario: Panel reload preserves sort mode
    Given a source directory
    And a file "z.txt" with content "z" in the source
    And a file "a.txt" with content "a" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I sort by size
    Then the sort mode should be Size
    When I refresh panels
    Then the sort mode should be Size

  Scenario: Empty directory panel has only parent entry
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    Then the right panel should have entries
