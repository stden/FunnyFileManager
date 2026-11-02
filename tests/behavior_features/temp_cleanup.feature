Feature: Temporary-file cleanup previews a narrow scope and requires an explicit Yes
  Background:
    Given an isolated acceptance workspace
    And an isolated temporary-file cleanup workspace

  Scenario: F9 previews only an old temporary file without cleaning unrelated caches
    When I open temporary-file cleanup from the F9 menu
    And the background temporary-file preview completes
    Then the rendered screen contains "Eligible files: 1 (128 B)"
    And the temporary cleanup preview includes only the old file and its bytes
    And all temporary cleanup fixture files remain unchanged

  Scenario Outline: Escape and the default Cancel button leave all files intact
    When I open temporary-file cleanup from the F9 menu
    And the background temporary-file preview completes
    And I dispatch the keys "<cancel>"
    Then the temporary cleanup dialog is closed
    And all temporary cleanup fixture files remain unchanged

    Examples:
      | cancel |
      | Esc    |
      | Enter  |

  Scenario: Enter on the default No returns to the preview without deleting files
    When I open temporary-file cleanup from the F9 menu
    And the background temporary-file preview completes
    And I dispatch the keys "Tab Enter"
    Then temporary cleanup confirmation defaults to No
    When I dispatch the keys "Enter"
    Then the temporary cleanup preview includes only the old file and its bytes
    And all temporary cleanup fixture files remain unchanged

  Scenario: An explicit Yes removes only the previewed old file and reports its logical size
    When I open temporary-file cleanup from the F9 menu
    And the background temporary-file preview completes
    And I dispatch the keys "Tab Enter"
    Then temporary cleanup confirmation defaults to No
    When I dispatch the keys "y"
    And the background temporary-file cleanup completes
    Then the rendered screen contains "Removed 128 B in 1 temporary files"
    And only the old temporary file has been removed

  Scenario: A file modified after preview is skipped during confirmed cleanup
    When I open temporary-file cleanup from the F9 menu
    And the background temporary-file preview completes
    And the old temporary file changes after preview
    And I dispatch the keys "Tab Enter y"
    And the background temporary-file cleanup completes
    Then the rendered screen contains "Removed 0 B in 0 temporary files"
    And the changed temporary file is retained

  Scenario: Modified confirmation shortcuts cannot authorize deletion
    When I open temporary-file cleanup from the F9 menu
    And the background temporary-file preview completes
    And I dispatch the keys "Tab Enter Ctrl+Y Ctrl+Enter Shift+Enter"
    Then temporary cleanup confirmation defaults to No
    And all temporary cleanup fixture files remain unchanged
    When I dispatch the keys "Esc"
    Then the temporary cleanup dialog is closed
    And all temporary cleanup fixture files remain unchanged
