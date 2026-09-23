# Purpose: BDD scenarios for the Far-style F7 make-folder dialog.
Feature: Make folder dialog
  As a file manager user
  I want F7 to create the folders I name
  So that I can organize files without leaving the manager

  Background:
    Given a source directory
    And an empty destination directory

  Scenario: F7 opens the dialog and creates the named folder
    Given the app is set up with source and destination
    When I create folder "reports" through F7
    Then "reports" should be a directory in the source

  Scenario: The cursor lands on the newly created folder
    Given a subdirectory "alpha" in the source
    And a subdirectory "omega" in the source
    And the app is set up with source and destination
    When I open the make folder dialog
    And I type folder name "middle"
    And I confirm the make folder dialog
    Then "middle" should be a directory in the source
    And the active panel selection should be "middle"

  Scenario: Process several folder names in one dialog
    Given the app is set up with source and destination
    When I open the make folder dialog
    And I type folder name "one;two;three"
    And I enable processing multiple folder names
    And I confirm the make folder dialog
    Then "one" should be a directory in the source
    And "two" should be a directory in the source
    And "three" should be a directory in the source
    And the active panel selection should be "one"
    And the status message should contain "3 folders"

  Scenario: A semicolon stays in a single name by default
    Given the app is set up with source and destination
    When I open the make folder dialog
    And I type folder name "one;two"
    And I confirm the make folder dialog
    Then "one;two" should be a directory in the source
    And "one" should not exist in the source
    And "two" should not exist in the source

  Scenario: An existing folder is not overwritten
    Given a subdirectory "taken" in the source with file "keep.txt"
    And the app is set up with source and destination
    When I open the make folder dialog
    And I type folder name "taken"
    And I confirm the make folder dialog
    Then "taken/keep.txt" should exist in the source
    And the status message should contain "already exists"

  Scenario: Cancelling the dialog creates nothing
    Given the app is set up with source and destination
    When I open the make folder dialog
    And I type folder name "unwanted"
    And I cancel the make folder dialog
    Then "unwanted" should not exist in the source
    And the make folder dialog should be closed
