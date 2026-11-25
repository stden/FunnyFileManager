# Purpose: BDD scenarios for copy and move operations.
Feature: Cross-panel copy and move operations
  As a file manager user
  I want to copy and move files between panels reliably
  So that I can organize my files across directories

  Scenario: Copy single file from left to right
    Given a source directory
    And a file "report.pdf" with content "pdf data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "report.pdf" in the left panel
    When I press F5
    Then "report.pdf" should exist in the destination
    And "report.pdf" should still exist in the source
    And the status message should contain "Copied"

  Scenario: Copy marked files ignores selection
    Given a source directory
    And a file "selected.txt" with content "sel" in the source
    And a file "marked_a.txt" with content "ma" in the source
    And a file "marked_b.txt" with content "mb" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "selected.txt" in the left panel
    And I mark "marked_a.txt" in the left panel
    And I mark "marked_b.txt" in the left panel
    When I press F5
    Then "marked_a.txt" should exist in the destination
    And "marked_b.txt" should exist in the destination
    And the status message should contain "Copied"

  Scenario: Move file removes from source
    Given a source directory
    And a file "transfer.dat" with content "payload" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "transfer.dat" in the left panel
    When I press F6
    Then "transfer.dat" should not exist in the source
    And "transfer.dat" should exist in the destination
    And the status message should contain "Moved"

  Scenario: Copy directory recursively
    Given a source directory
    And a subdirectory "project" in the source
    And a file "project/main.rs" with content "fn main() {}" in the source dir
    And a file "project/lib.rs" with content "pub fn run() {}" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    And I select "project" in the left panel
    When I press F5
    Then "project" should still exist in the source
    And the status message should contain "Copied"

  Scenario: Copy creates unique name on conflict
    Given a source directory
    And a file "clash.txt" with content "new version" in the source
    And an empty destination directory
    And a file "clash.txt" with content "old version" in the destination
    And the app is set up with source and destination
    And I select "clash.txt" in the left panel
    When I press F5
    Then "clash.txt" should still exist in the source
    And the status message should contain "Copied"

  Scenario: Move directory with nested files
    Given a source directory
    And a subdirectory "movedir" in the source
    And a file "movedir/inner.txt" with content "deep" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    And I select "movedir" in the left panel
    When I press F6
    Then "movedir" should not exist in the source
    And "movedir" should exist in the destination
    And the status message should contain "Moved"

  Scenario: Copy then move the same file
    Given a source directory
    And a file "workflow.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "workflow.txt" in the left panel
    When I press F5
    Then "workflow.txt" should exist in the destination
    And "workflow.txt" should still exist in the source
    When I select "workflow.txt" in the left panel
    And I press F6
    Then "workflow.txt" should not exist in the source
