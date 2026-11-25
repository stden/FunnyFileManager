# Purpose: BDD scenarios for creating files through dialogs.
Feature: New file dialog (Shift+F4)
  As a file manager user
  I want to create new files via a dialog
  So that I can quickly create files without leaving the manager

  Scenario: Create a new file via Shift+F4
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename "newfile.txt"
    And I confirm new file
    Then the status message should contain "Created"

  Scenario: Empty filename is rejected
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename ""
    And I confirm new file
    Then the status message should contain "empty"

  Scenario: Whitespace-only filename is rejected
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename "   "
    And I confirm new file
    Then the status message should contain "empty"

  Scenario: Existing filename is rejected
    Given a source directory
    And a file "exists.txt" with content "already here" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename "exists.txt"
    And I confirm new file
    Then the status message should contain "already exists"

  Scenario: Create file with extension
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename "script.sh"
    And I confirm new file
    Then the status message should contain "Created"
