Feature: Far-style directory reading progress and Esc interruption
  When reading a large or slow directory takes longer than 100 ms, the panel
  being read displays " Reading: <N> files " in teal and black centered across
  its top double-line border, and pressing Esc stops the walk immediately so
  the user sees the partial list of files read so far alongside "..".

  Scenario: Slow read on the left panel renders the teal Reading banner on the left top border
    Given an isolated acceptance workspace
    And a subfolder "huge-left" under the left panel with 5 files
    When I enter "huge-left" on the left panel with immediate reading progress
    Then the reading progress banner shows " Reading: 5 files " on the left panel top border

  Scenario: Slow read on the right panel renders the teal Reading banner on the right top border
    Given an isolated acceptance workspace
    And a subfolder "huge-right" under the right panel with 4 files
    When I switch to the right panel and enter "huge-right" with immediate reading progress
    Then the reading progress banner shows " Reading: 4 files " on the right panel top border

  Scenario: Fast directory read finishes without painting a Reading banner
    Given an isolated acceptance workspace
    And a subfolder "quick" under the left panel with 3 files
    When I enter "quick" on the left panel within the 100 ms threshold
    Then no reading progress banner was painted

  Scenario: Pressing Esc during a slow directory read keeps the partial file list
    Given an isolated acceptance workspace
    And a subfolder "interrupted" under the left panel with 6 files
    When I enter "interrupted" on the left panel and press Esc after 3 files
    Then the left panel is inside "interrupted" showing ".." and 3 files
