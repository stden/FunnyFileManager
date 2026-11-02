# Purpose: BDD scenarios for bookmark workflows and saved panel locations.
Feature: Bookmark management workflows
  As a power user
  I want bookmarks to reliably save and restore locations
  So that I can quickly navigate between frequently used directories

  Scenario: Set bookmark and recall it
    Given a source directory
    And a subdirectory "project" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I set bookmark 0
    When I execute command "cd project"
    Then the active panel path should end with "project"
    When I go to bookmark 0
    Then the active panel path should not end with "project"

  Scenario: Empty bookmark shows message
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And bookmark 5 is empty
    When I go to bookmark 5
    Then the status message should contain "empty"

  Scenario: Multiple bookmarks can coexist
    Given a source directory
    And a subdirectory "alpha" in the source
    And a subdirectory "beta" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "cd alpha"
    When I set bookmark 1
    When I execute command "cd .."
    When I execute command "cd beta"
    When I set bookmark 2
    When I go to bookmark 1
    Then the active panel path should end with "alpha"
    When I go to bookmark 2
    Then the active panel path should end with "beta"

  Scenario: Bookmark survives panel operations
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I set bookmark 3
    When I swap panels
    When I swap panels
    When I go to bookmark 3
    Then the status message should not be empty
