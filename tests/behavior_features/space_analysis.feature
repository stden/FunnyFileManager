Feature: Space analysis judges each entry of a folder against git and removes nothing
  Background:
    Given an isolated acceptance workspace
    And a git repository in the left panel with a tracked file, an ignored target folder and an untracked note

  Scenario: Commands > Space analysis opens a dialog on the folder with its totals
    When I choose Space analysis from the F9 menu
    Then the Space analysis dialog is open
    And the Space analysis dialog names the left folder
    And the rendered screen contains "Reclaimable safely"

  Scenario: Git decides each verdict: tracked files are kept, the ignored cache is safe, the untracked note needs review
    When I choose Space analysis from the F9 menu
    Then the Space analysis labels "tracked.txt" as "KEEP"
    And the Space analysis labels "target" as "SAFE"
    And the Space analysis labels "notes.txt" as "REVIEW"
    And the Space analysis reclaims "4.00 KB"

  Scenario: Enter selects the highlighted entry in the active panel and closes the dialog
    When I choose Space analysis from the F9 menu
    And I move the Space analysis cursor to "notes.txt"
    And I dispatch the keys "Enter"
    Then the overlay is closed
    And the left panel cursor selects "notes.txt"
    And every file of the repository fixture still exists

  Scenario: Esc closes the dialog and removes nothing
    When I choose Space analysis from the F9 menu
    And I dispatch the keys "Esc"
    Then the overlay is closed
    And every file of the repository fixture still exists

  Scenario: A NetBox panel refuses Space analysis, and the reason is on screen
    Given the active panel shows a NetBox sessions list
    When I choose Space analysis from the F9 menu
    Then the rendered screen contains "NetBox: this command works on files; press Enter on `..`"
    And the overlay is closed
