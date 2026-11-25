# Purpose: BDD scenarios for Far-style directory reading progress and Esc interruption.

Feature: Long directory reading progress and Esc partial listing
  As a user opening a directory with hundreds of thousands of files
  I want Far-style " Reading: N files " progress in the panel top border and Esc interruption
  So that I can see reading is alive and stop early with the partial file list

  Scenario: Long directory read paints the teal Reading banner in the active panel border
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And a folder "large-set" in the source with 5 numbered files
    When I render the app and enter "large-set" with immediate reading progress
    Then the reading banner shows " Reading: 5 files " on the left panel border

  Scenario: Pressing Esc during directory reading keeps the partial file list and parent entry
    Given a temporary directory
    And 6 numbered files
    And a panel at the temporary directory
    When I reload the panel and interrupt reading after 3 files
    Then the panel lists ".." and 3 files
