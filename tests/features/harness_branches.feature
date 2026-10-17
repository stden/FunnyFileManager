# Purpose: Scenarios for step branches the other features never take.
@harness
Feature: Step branches the other features leave idle
  As a maintainer of the Cucumber harness
  I want every step's success and rejection path to run
  So that a green suite executes the step definitions

  Scenario: A subdirectory panel, marks, a bookmark and both hide toggles
    Given a temporary directory
    And a file "right.txt" with content "r"
    And a subdirectory "room" with file "a.txt"
    And a panel at "room" in the temporary directory
    Then the panel should have 2 entries
    And entry 9 is outside the panel
    When I create a panel at the temporary directory
    Given an app at "room" in the temporary directory
    And I select "right.txt" in the right panel
    And the file "a.txt" is marked
    Then "a.txt" should be marked in the app
    And "right.txt" should not be marked in the app
    And the left panel should have a valid selection
    And the left panel path should differ from original
    When I set bookmark 1
    Then bookmark 1 should be set to "room"
    When I switch to the right panel
    Then the active panel should be Right
    When I toggle hide left panel
    Then help should be visible
    When I toggle hide left panel
    Then help should not be visible
    When I toggle hide right panel
    Then drive selection should be visible
    When I toggle hide right panel
    Then help should not be visible
    And the diagnostic "Not a tool" is not a Windows tool
    And an empty event stream exhausts the key workflow
    And a missing entry is named in each panel
    And a missing entry is named on either side

  Scenario: Parent of the filesystem root is not a directory
    Given a source directory
    And a subdirectory "room" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I navigate to "room"
    Then the left panel path should differ from original
    When I go to root
    And I go to parent directory
    Then the active panel should be at the source filesystem root

  Scenario: The panel path check uses the app when no panel is open
    Given a temporary directory
    And a subdirectory "nested" with file "a.txt"
    And an app at "nested" in the temporary directory
    Then the panel path should end with "nested"

  Scenario: Inserting into a command line that already holds text
    Given a temporary directory
    And a file "note.txt" with content "n"
    And an app at the temporary directory
    And I select "note.txt" in the left panel
    And the command line holds "echo"
    When I insert filename into command line
    And I insert full path into command line
    Then the command line should contain "echo"
    And the command line should contain "note.txt"
    Given the left panel has no selection
    When I insert filename into command line
    And I insert full path into command line

  Scenario: Creating a file from the active panel
    Given a temporary directory
    And an app at the temporary directory
    When I create new file "fresh.txt" in the active panel
    Then "fresh.txt" should exist in the temporary directory

  Scenario: Sort mode names that are not modes, and an extension check with one row
    Given a temporary directory
    And a panel at the temporary directory
    Then the first extension should come before the last alphabetically
    And the sort mode Bogus is rejected
    When the panel lists no entries
    And I press PageUp
    And I press PageDown

  Scenario: Local paths are not S3 paths, and profile discovery keeps default
    When I parse path "/srv/example"
    Then a local path is not an S3 path
    When I list AWS profiles
    Then the profiles should include "default"

  Scenario: An existing-file policy outside the dialog is rejected
    Then the existing-file policy "Neither" is rejected

  Scenario: A write aimed at the filesystem root is rejected
    Given a temporary directory
    And a source directory
    And a write at the filesystem root is rejected

  Scenario: Fixture failures name the file they could not use
    Given a source directory
    And an empty destination directory
    Then a missing age fixture names the file
    Given a file "note.txt" with content "hello" in the source
    Then a missing line in "note.txt" names that file
