# Purpose: BDD scenarios for viewer behavior.
Feature: File viewer scenarios
  As a file manager user
  I want the built-in viewer to correctly display file contents
  So that I can inspect files without leaving the file manager

  Scenario: View a text file shows its contents
    Given a source directory
    And a file "readme.txt" with content "Hello World" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "readme.txt" in the left panel
    When I press F3
    Then the viewer should be open
    And the viewer title should contain "readme.txt"
    And the viewer should show "Hello World"

  Scenario: Close viewer returns to panel
    Given a source directory
    And a file "temp.txt" with content "temporary" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "temp.txt" in the left panel
    When I press F3
    Then the viewer should be open
    When I close the viewer
    Then the viewer should not be open

  Scenario: View a directory shows error
    Given a source directory
    And a subdirectory "noview" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "noview" in the left panel
    When I press F3
    Then the viewer should not be open

  Scenario: View with no selection does nothing
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And the left panel has no selection
    When I press F3
    Then the viewer should not be open

  Scenario: View file then copy it
    Given a source directory
    And a file "viewcopy.txt" with content "check then copy" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "viewcopy.txt" in the left panel
    When I press F3
    Then the viewer should be open
    And the viewer should show "check then copy"
    When I close the viewer
    When I select "viewcopy.txt" in the left panel
    And I press F5
    Then "viewcopy.txt" should exist in the destination

  Scenario: View file with multiline content
    Given a source directory
    And a file "multi.txt" with content "line one and line two" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "multi.txt" in the left panel
    When I press F3
    Then the viewer should be open
    And the viewer should show "line one"
