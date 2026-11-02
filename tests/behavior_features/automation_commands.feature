Feature: Named automation commands execute through typed modal interpreters
  Background:
    Given an isolated acceptance workspace

  Scenario: Named panel viewer and editor commands run through their active contexts
    Given a file named "story.txt" with 40 numbered lines
    When I invoke automation command "panels" "view"
    Then the active automation context is "viewer"
    When I invoke automation command "viewer" "page_down"
    Then the viewer has scrolled past its first line
    When I invoke automation command "viewer" "close"
    Then the overlay is closed
    And the active automation context is "panels"
    When I invoke automation command "panels" "edit"
    Then the active automation context is "editor"
    When I invoke automation command "editor" "newline"
    And I invoke automation command "editor" "save"
    And I invoke automation command "editor" "close"
    Then the overlay is closed

  Scenario: Named command-line and task-list commands respect nested modal boundaries
    When I dispatch the keys ":"
    Then the active automation context is "command_line"
    When I invoke automation command "command_line" "history_search"
    Then the active automation context is "command_line_search"
    And automation command "command_line" "submit" is rejected for the current context
    When I invoke automation command "command_line_search" "cancel"
    Then the active automation context is "command_line"
    When I dispatch the keys "Esc Ctrl+W"
    Then the active automation context is "task_list"
    When I invoke automation command "task_list" "toggle_tab"
    Then the task list displays the "Network" tab
    When I invoke automation command "task_list" "close"
    Then the overlay is closed

  Scenario: Out-of-context and unknown automation lookups fail without mutating state
    Given a file named "safe.txt" containing "untouched"
    Then automation lookup "unknown_ctx" "view" fails with UnknownContext
    And automation lookup "panels" "no_such_action" fails with UnknownAction
    And automation command "editor" "save" is rejected for the current context
    When I invoke automation command "panels" "edit"
    Then automation command "panels" "copy" is rejected for the current context
    And the editor buffer equals "untouched"
