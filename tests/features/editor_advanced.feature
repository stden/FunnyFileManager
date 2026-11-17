# Purpose: BDD scenarios for editor workflows.
Feature: Advanced editor scenarios
  As a file manager user
  I want the editor to handle all edge cases
  So that editing never corrupts my workflow

  Scenario: F4 on a directory shows error
    Given a source directory
    And a subdirectory "noEdit" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "noEdit" in the left panel
    When I press F4 with success behavior
    Then the status message should contain "directories"

  Scenario: F4 with no selection does nothing
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And the left panel has no selection
    When I press F4 with success behavior
    Then the viewer should not be open

  Scenario: F4 on parent entry does nothing
    Given a source directory
    And a file "dummy.txt" with content "d" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select ".." in the left panel
    When I press F4 with success behavior
    Then the status message should not contain "Error"

  Scenario: F4 fail launch reports error
    Given a source directory
    And a file "edit_me.txt" with content "text" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "edit_me.txt" in the left panel
    When I press F4 with fail_launch behavior
    Then the status message should contain "Failed"

  Scenario: F4 exit error reports error
    Given a source directory
    And a file "edit_me.txt" with content "text" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "edit_me.txt" in the left panel
    When I press F4 with exit_error behavior
    Then the status message should contain "exited"

  Scenario: F4 from right panel works
    Given a source directory
    And a file "right_file.txt" with content "right" in the source
    And an empty destination directory
    And a file "right_edit.txt" with content "edit this" in the destination
    And the app is set up with source and destination
    When I set the active panel to Right
    And I select "right_edit.txt" in the right panel
    When I press F4 with success behavior
    Then the status message should not contain "Error"

  Scenario: Edit a file then copy it to other panel
    Given a source directory
    And a file "workflow.txt" with content "original" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "workflow.txt" in the left panel
    When I press F4 with success behavior
    And I select "workflow.txt" in the left panel
    When I press F5
    Then "workflow.txt" should exist in the destination
    And "workflow.txt" should still exist in the source
