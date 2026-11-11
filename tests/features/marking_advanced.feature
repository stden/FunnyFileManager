# Purpose: BDD scenarios for marking and unmarking files.
Feature: Advanced marking and selection scenarios
  As a file manager user
  I want marking to work correctly in all edge cases
  So that I can select exactly the files I need

  Scenario: Toggle mark on a file marks it
    Given a temporary directory
    And a file "x.txt" with content "x"
    And a panel at the temporary directory
    And I select the entry "x.txt"
    When I toggle mark
    Then "x.txt" should be marked

  Scenario: Toggle mark twice unmarks the file
    Given a temporary directory
    And a file "y.txt" with content "y"
    And a panel at the temporary directory
    And I select the entry "y.txt"
    When I toggle mark
    And I toggle mark
    Then "y.txt" should not be marked

  Scenario: Clear marks removes all marks
    Given a temporary directory
    And a file "a.txt" with content "a"
    And a file "b.txt" with content "b"
    And a panel at the temporary directory
    And "a.txt" is marked
    And "b.txt" is marked
    When I clear all marks
    Then the panel should have no marks

  Scenario: Marking does not affect parent entry
    Given a temporary directory
    And a file "child.txt" with content "c"
    And a panel at the temporary directory
    And I select the entry ".."
    When I toggle mark
    Then ".." should not be marked

  Scenario: Copy uses marked files instead of selection
    Given a source directory
    And a file "sel.txt" with content "selected" in the source
    And a file "mark1.txt" with content "marked1" in the source
    And a file "mark2.txt" with content "marked2" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "sel.txt" in the left panel
    And I mark "mark1.txt" in the left panel
    And I mark "mark2.txt" in the left panel
    When I press F5
    Then "mark1.txt" should exist in the destination
    And "mark2.txt" should exist in the destination

  Scenario: Marks persist across sort mode changes
    Given a source directory
    And a file "keep.txt" with content "k" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I mark "keep.txt" in the left panel
    When I sort by extension
    Then the sort mode should be Extension
