# Purpose: BDD scenarios for parallel single-pass delete progress and Esc interruption.

Feature: Parallel streaming directory deletion and Esc cancellation
  As a user deleting large folder trees with F8
  I want deletion to start immediately in parallel with streaming progress and Esc cancellation
  So that large folders vanish rapidly without a double directory scan and Esc stops safely

  Scenario: Deleting a large folder streams file counts directly in Working phase
    Given a source directory
    And an empty destination directory
    And a folder "bulk-tree" in the source with 40 numbered files
    And the app is set up with source and destination
    And I select "bulk-tree" in the left panel
    When I delete the selection with immediate progress capture
    Then the delete progress streamed 40 files in working phase without scanning
    And "bulk-tree" does not exist in the source

  Scenario: Pressing Esc during a batch deletion stops cleanly without an error dialog
    Given a source directory
    And an empty destination directory
    And a file "aa_gone.txt" with content "1" in the source
    And a file "zz_kept.txt" with content "2" in the source
    And the app is set up with source and destination
    And I mark "aa_gone.txt" in the left panel
    And I mark "zz_kept.txt" in the left panel
    When I press F8 and interrupt deletion with Esc after the first progress paint
    Then deletion stops cleanly with "Delete cancelled" and keeps "zz_kept.txt" in the source
