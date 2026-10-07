# Purpose: Drives the Far-style F5/F6 dialog through realistic collision choices in TempDir.
Feature: Transfer dialog policies
  As a file manager user
  I want explicit control over existing files and destinations
  So that copying and moving never changes the wrong data

  Background:
    Given a source directory
    And an empty destination directory

  Scenario: Ask, answered Keep both, keeps the existing file and creates a uniquely named copy
    Given a file "report.txt" with content "draft" in the source
    And a file "report.txt" with content "approved" in the destination
    And the app is set up with source and destination
    And I select "report.txt" in the active panel
    When I copy the selection through F5 asking, and keep both
    Then "report.txt" in the destination should have content "approved"
    And "report_1.txt" in the destination should have content "draft"
    And "report.txt" in the source should have content "draft"

  Scenario: Skip leaves both copies untouched
    Given a file "invoice.txt" with content "new draft" in the source
    And a file "invoice.txt" with content "booked" in the destination
    And the app is set up with source and destination
    And I select "invoice.txt" in the active panel
    When I copy the selection through F5 using "Skip"
    Then "invoice.txt" in the destination should have content "booked"
    And "invoice.txt" in the source should have content "new draft"
    And "invoice_1.txt" should not exist in the destination

  Scenario: Overwrite replaces an existing destination file
    Given a file "schedule.txt" with content "current" in the source
    And a file "schedule.txt" with content "obsolete" in the destination
    And the app is set up with source and destination
    And I select "schedule.txt" in the active panel
    When I copy the selection through F5 using "Overwrite"
    Then "schedule.txt" in the destination should have content "current"
    And "schedule_1.txt" should not exist in the destination

  Scenario: Only newer files skips an older source
    Given a file "catalog.txt" with content "old catalog" in the source
    And a file "catalog.txt" with content "new catalog" in the destination
    And the source copy of "catalog.txt" is older than the destination copy
    And the app is set up with source and destination
    And I select "catalog.txt" in the active panel
    When I copy the selection through F5 using "Only newer files"
    Then "catalog.txt" in the destination should have content "new catalog"

  Scenario: Only newer files accepts a newer source
    Given a file "prices.txt" with content "September prices" in the source
    And a file "prices.txt" with content "August prices" in the destination
    And the source copy of "prices.txt" is newer than the destination copy
    And the app is set up with source and destination
    And I select "prices.txt" in the active panel
    When I copy the selection through F5 using "Only newer files"
    Then "prices.txt" in the destination should have content "September prices"

  Scenario: Escape cancels without copying
    Given a file "private.txt" with content "keep local" in the source
    And the app is set up with source and destination
    And I select "private.txt" in the active panel
    When I cancel the F5 copy dialog
    Then "private.txt" in the source should have content "keep local"
    And "private.txt" should not exist in the destination

  Scenario: A relative destination is resolved from the active panel
    Given a file "notes.txt" with content "field notes" in the source
    And the app is set up with source and destination
    And I select "notes.txt" in the active panel
    When I copy the selection through F5 to relative destination "archive/2026/"
    Then "archive/2026/notes.txt" in the source should have content "field notes"
    And "notes.txt" should not exist in the destination

  # Far's rule: a destination that is not an existing folder and does not end
  # in a separator is the new name of the one item copied.
  Scenario: A typed name without a trailing separator is the copy's new name
    Given a file "notes.txt" with content "field notes" in the source
    And the app is set up with source and destination
    And I select "notes.txt" in the active panel
    When I copy the selection through F5 to relative destination "archive/notes-2026.txt"
    Then "archive/notes-2026.txt" in the source should have content "field notes"
    And "notes.txt" in the source should have content "field notes"
    And "notes.txt" should not exist in the destination

  Scenario: Skipping a move collision keeps the source file
    Given a file "contract.txt" with content "unsigned" in the source
    And a file "contract.txt" with content "signed" in the destination
    And the app is set up with source and destination
    And I select "contract.txt" in the active panel
    When I move the selection through F6 using "Skip"
    Then "contract.txt" in the source should have content "unsigned"
    And "contract.txt" in the destination should have content "signed"
