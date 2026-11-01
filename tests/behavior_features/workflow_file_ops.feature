Feature: File operation dialogs open, execute and close cleanly

  Background:
    Given an isolated acceptance workspace
    And the left panel contains the file "report.txt"
    And the left panel contains the file "old-name.txt"
    And the left panel contains the file "data.csv"

  Scenario: Copy dialog appears on F5 and closes on Esc
    Given "report.txt" is selected in the left panel
    When I dispatch the keys "F5"
    Then the rendered screen contains "Copy"
    When I dispatch the keys "Esc"
    Then the overlay is closed

  Scenario: Delete dialog appears on F8 and closes on Esc
    Given "report.txt" is selected in the left panel
    When I dispatch the keys "F8"
    Then the rendered screen contains "Delete"
    When I dispatch the keys "Esc"
    Then the overlay is closed

  Scenario: Rename dialog appears on Shift+F6 and closes on Esc
    Given "old-name.txt" is selected in the left panel
    When I dispatch the keys "Shift+F6"
    Then the rename prompt targets "old-name.txt"
    And the rendered screen contains "Rename"
    When I dispatch the keys "Esc"
    Then the overlay is closed

  Scenario: F7 creates a folder and settles the cursor on it
    When I dispatch the keys "F7"
    And I type the text "build-output"
    And I dispatch the keys "Enter"
    Then the overlay is closed
    And the left panel directory "build-output" exists on disk
    And the left panel cursor selects "build-output"

  Scenario: F5 copies the selected file to the right panel directory
    Given the left file "report.txt" contains "quarterly metrics"
    And "report.txt" is selected in the left panel
    When I dispatch the keys "F5 Enter"
    Then the overlay is closed
    And the left file "report.txt" equals "quarterly metrics"
    And the right file "report.txt" equals "quarterly metrics"

  Scenario: Shift+F6 renames the selected file in place
    Given the left file "old-name.txt" contains "keep this body"
    And "old-name.txt" is selected in the left panel
    When I dispatch the keys "Shift+F6 Ctrl+U"
    And I type the text "renamed.txt"
    And I dispatch the keys "Enter"
    Then the overlay is closed
    And the left file "old-name.txt" is absent on disk
    And the left file "renamed.txt" equals "keep this body"
    And the left panel cursor selects "renamed.txt"

  Scenario: F6 moves the selected file to the right panel directory
    Given the left file "data.csv" contains "a,b,c"
    And "data.csv" is selected in the left panel
    When I dispatch the keys "F6 Enter"
    Then the overlay is closed
    And the left file "data.csv" is absent on disk
    And the right file "data.csv" equals "a,b,c"

  Scenario: Confirmed F8 removes the selected file from disk and listing
    Given "report.txt" is selected in the left panel
    When I dispatch the keys "F8 y"
    Then the overlay is closed
    And the left file "report.txt" is absent on disk
    And the rendered screen excludes "report.txt"
