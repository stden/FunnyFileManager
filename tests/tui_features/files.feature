Feature: File operations through visible dialogs
  Scenario: Copy a file and cancel then confirm deletion
    Given a TUI workspace
    And the left file "notes.txt" contains "Copy this payload"
    When I open the TUI at 100 by 30
    And I press "Down F5"
    Then the screen contains "Copy"
    When I press "Enter"
    Then the right file "notes.txt" contains "Copy this payload"
    And the right panel shows "notes.txt"
    When I press "Tab End F8"
    Then the screen contains "Delete"
    When I press "Esc"
    Then the right file "notes.txt" contains "Copy this payload"
    When I press "F8 Enter"
    Then the right file "notes.txt" does not exist
    And the left file "notes.txt" contains "Copy this payload"
    When I press "F10 Enter"
    Then the TUI exits successfully

  Scenario: Create a directory with spaces through F7
    Given a TUI workspace
    When I open the TUI at 100 by 30
    And I press "F7"
    Then the screen contains "folder"
    When I type "new folder"
    And I press "Enter"
    Then the left directory "new folder" exists
    And the left panel shows "new folder"
    When I press "F10 Enter"
    Then the TUI exits successfully
