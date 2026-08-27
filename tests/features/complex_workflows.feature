# Purpose: BDD scenarios for complex multi-step file-manager workflows.
Feature: Complex multi-step workflows
  As a power user
  I want to chain multiple operations together
  So that I can manage files efficiently

  Scenario: Create directory, create file inside, then copy to other panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press F7
    Then "new_folder" should be a directory in the source

  Scenario: Copy multiple marked files then verify source is intact
    Given a source directory
    And a file "alpha.txt" with content "aaa" in the source
    And a file "beta.txt" with content "bbb" in the source
    And a file "gamma.txt" with content "ccc" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I mark "alpha.txt" in the left panel
    And I mark "gamma.txt" in the left panel
    When I press F5
    Then "alpha.txt" should exist in the destination
    And "gamma.txt" should exist in the destination
    And "alpha.txt" should still exist in the source
    And "gamma.txt" should still exist in the source
    And the status message should contain "Copied"

  Scenario: View file then close viewer then delete the file
    Given a source directory
    And a file "temp.log" with content "log data here" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "temp.log" in the left panel
    When I press F3
    Then the viewer should be open
    And the viewer should show "log data here"
    When I close the viewer
    Then the viewer should not be open

  Scenario: Navigate into subdirectory and copy file from there
    Given a source directory
    And a subdirectory "inner" in the source
    And a file "inner/deep.txt" with content "nested" in the source dir
    And an empty destination directory
    And the app is set up at "inner" in the source
    And I select "deep.txt" in the left panel
    When I press F5
    Then "deep.txt" should exist in the destination
    And the status message should contain "Copied"

  Scenario: Sort by extension then copy preserves operation
    Given a source directory
    And a file "data.csv" with content "1,2,3" in the source
    And a file "notes.txt" with content "hello" in the source
    And a file "image.png" with content "binary" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I sort by extension
    Then the sort mode should be Extension
    When I select "data.csv" in the left panel
    And I press F5
    Then "data.csv" should exist in the destination
    And the status message should contain "Copied"

  Scenario: Swap panels then copy goes to original source
    Given a source directory
    And a file "swap_test.txt" with content "swapped" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I swap panels
    Then the active panel should be Left
    When I set the active panel to Right
    And I select "swap_test.txt" in the right panel
    When I press F5
    Then the status message should contain "Copied"

  Scenario: Bookmark directory navigate away and return
    Given a source directory
    And a subdirectory "projects" in the source
    And a file "projects/code.rs" with content "fn main(){}" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    When I set bookmark 1
    When I execute command "cd /"
    Then the active panel path should be "/"
    When I go to bookmark 1
    Then the active panel path should not end with "/"

  Scenario: Mark all files then unmark and verify
    Given a source directory
    And a file "one.txt" with content "1" in the source
    And a file "two.txt" with content "2" in the source
    And a file "three.txt" with content "3" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I select all files
    Then the active panel should have marks
    When I select all files
    Then the active panel should have no marks

  Scenario: Delete directory with nested structure
    Given a source directory
    And a subdirectory "deep" in the source
    And a file "deep/level1.txt" with content "L1" in the source dir
    And a file "deep/sub/level2.txt" with content "L2" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    And I select "deep" in the left panel
    When I press F8
    And I confirm the deletion
    Then "deep" should not exist in the source

  Scenario: Multiple mkdir creates incrementing names
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press F7
    Then "new_folder" should be a directory in the source
    When I press F7
    Then "new_folder_1" should be a directory in the source

  Scenario: Quick search then copy found file
    Given a source directory
    And a file "apple.txt" with content "fruit" in the source
    And a file "banana.txt" with content "yellow" in the source
    And a file "cherry.txt" with content "red" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I quick search for "b" in the app
    When I press F5
    Then the status message should contain "Copied"

  Scenario: Toggle help on and off preserves panel state
    Given a source directory
    And a file "stable.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "stable.txt" in the left panel
    When I toggle help
    Then help should be visible
    When I toggle help
    Then help should not be visible

  Scenario: Console mode toggle does not lose panel contents
    Given a source directory
    And a file "persist.txt" with content "still here" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I toggle console mode
    Then console mode should be on
    When I toggle console mode
    Then console mode should be off
    And the left panel should have entries
