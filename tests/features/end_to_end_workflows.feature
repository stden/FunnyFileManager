# Purpose: BDD scenarios for end-to-end workflows across app features.
Feature: End-to-end user workflows
  As a typical file manager user
  I want to perform complete workflows
  So that I can accomplish real-world file management tasks

  Scenario: Organize project files into directories
    Given a source directory
    And a file "main.rs" with content "fn main() {}" in the source
    And a file "lib.rs" with content "pub fn run() {}" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I press F7
    Then "new_folder" should be a directory in the source

  Scenario: Browse, view, and copy a file
    Given a source directory
    And a file "config.yaml" with content "key: value" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "config.yaml" in the left panel
    When I press F3
    Then the viewer should be open
    And the viewer should show "key: value"
    When I close the viewer
    When I select "config.yaml" in the left panel
    And I press F5
    Then "config.yaml" should exist in the destination

  Scenario: Clean up by deleting temporary files
    Given a source directory
    And a file "temp1.tmp" with content "t1" in the source
    And a file "temp2.tmp" with content "t2" in the source
    And a file "keep.txt" with content "important" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I mark "temp1.tmp" in the left panel
    And I mark "temp2.tmp" in the left panel
    When I press F8
    And I confirm the deletion
    Then "temp1.tmp" should not exist in the source
    And "temp2.tmp" should not exist in the source
    And "keep.txt" should still exist in the source

  Scenario: Navigate deep, bookmark, and return
    Given a source directory
    And a subdirectory "level1" in the source
    And a file "level1/level2/deep.txt" with content "deep" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "cd level1"
    When I set bookmark 9
    When I go to root
    Then the active panel should be at the source filesystem root
    When I go to bookmark 9
    Then the active panel path should end with "level1"

  Scenario: Sort files then select and copy specific one
    Given a source directory
    And a file "small.txt" with content "s" in the source
    And a file "medium.txt" with content "mmmmm" in the source
    And a file "large.txt" with content "llllllllll" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I sort by name
    Then the sort mode should be Name
    When I select "medium.txt" in the left panel
    And I press F5
    Then "medium.txt" should exist in the destination

  Scenario: Mark all, unmark, then mark specific
    Given a source directory
    And a file "a.txt" with content "a" in the source
    And a file "b.txt" with content "b" in the source
    And a file "c.txt" with content "c" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I select all files
    Then the active panel should have marks
    When I select all files
    Then the active panel should have no marks
    And I mark "b.txt" in the left panel
    When I press F5
    Then "b.txt" should exist in the destination
    And the status message should contain "Copied"

  Scenario: Create file via dialog then view it
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename "created.txt"
    And I confirm new file
    Then the status message should contain "Created"

  Scenario: Move file then verify panels refreshed
    Given a source directory
    And a file "movable.txt" with content "move me" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "movable.txt" in the left panel
    When I press F6
    Then "movable.txt" should not exist in the source
    And "movable.txt" should exist in the destination
    And the left panel should have entries
    And the right panel should have entries

  Scenario: Switch panels and work from right panel
    Given a source directory
    And a file "right_side.txt" with content "right" in the source
    And an empty destination directory
    And a file "right_target.txt" with content "target" in the destination
    And the app is set up with source and destination
    When I set the active panel to Right
    And I select "right_target.txt" in the right panel
    When I press F5
    Then the status message should contain "Copied"
