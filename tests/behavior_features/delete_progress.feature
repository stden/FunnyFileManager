Feature: Parallel streaming delete progress and Esc interruption
  Deleting large trees starts unlinking in parallel immediately in Working
  phase, renders the Delete progress dialog across viewports without following
  directory links, suppresses the dialog below 100 ms, and handles Esc cleanly.

  Scenario: Confirmed F8 on a nested folder streams Working progress, renders the Delete dialog, and keeps link targets
    Given an isolated acceptance workspace
    And a nested folder "build-cache" under the left panel with 36 files and a directory link "linked-docs" keeping "kept-docs"
    And "build-cache" is selected in the left panel
    When I dispatch "F8 Enter" with immediate delete progress capture
    Then the left file "build-cache" is absent on disk
    And the kept target "kept-docs" still exists under the left panel
    And all 37 deleted items streamed in working phase and render the Delete dialog on normal and compact viewports
    And the rendered screen excludes "build-cache"
    And performance statistics record "Delete/Remove" and "Delete/Refresh"

  Scenario: Fast F8 folder deletion within 100 ms paints no progress dialog
    Given an isolated acceptance workspace
    And a nested folder "tiny-cache" under the left panel with 4 files
    And "tiny-cache" is selected in the left panel
    When I dispatch "F8 Enter" within the 100 ms delete threshold
    Then the left file "tiny-cache" is absent on disk
    And no delete progress frame was painted

  Scenario: Pressing Esc during a shown multi-file deletion stops cleanly without an error dialog
    Given an isolated acceptance workspace
    And left panel entries "aa_temp.log" and "zz_keep.log" are marked
    When I press F8 and interrupt deletion via headless cancel
    Then the left file "aa_temp.log" is absent on disk
    And the left file "zz_keep.log" still exists on disk and marks are cleared
    And the overlay is closed
    And the rendered screen contains "Delete cancelled"
