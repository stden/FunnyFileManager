# Purpose: BDD scenarios for command-line parsing and execution behavior.
Feature: Advanced command line scenarios
  As a file manager user
  I want the command line to handle edge cases
  So that I get clear feedback for any input

  Scenario: cd to a valid subdirectory
    Given a source directory
    And a subdirectory "target_dir" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "cd target_dir"
    Then the active panel path should end with "target_dir"

  Scenario: cd without argument shows error
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "cd"
    Then the status message should contain "requires a path"

  Scenario: cd to nonexistent directory shows error
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "cd nonexistent_path_12345"
    Then the status message should contain "Error"

  Scenario: sort name sets Name mode
    Given a source directory
    And a file "b.txt" with content "b" in the source
    And a file "a.txt" with content "a" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "sort name"
    Then the sort mode should be Name

  Scenario: sort ext sets Extension mode
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "sort ext"
    Then the sort mode should be Extension

  Scenario: sort size sets Size mode
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "sort size"
    Then the sort mode should be Size

  Scenario: sort with invalid argument shows error
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "sort blah"
    Then the status message should contain "expects"

  Scenario: bookmark set and go roundtrip
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I set bookmark 3
    When I execute command "cd /"
    When I go to bookmark 3
    Then the active panel path should not end with "/"

  Scenario: go to empty bookmark shows message
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And bookmark 7 is empty
    When I go to bookmark 7
    Then the status message should contain "empty"

  Scenario: mark all command marks all files
    Given a source directory
    And a file "x.txt" with content "x" in the source
    And a file "y.txt" with content "y" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "mark all"
    Then the active panel should have marks

  Scenario: unmark command clears all marks
    Given a source directory
    And a file "x.txt" with content "x" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "mark all"
    Then the active panel should have marks
    When I execute command "unmark"
    Then the active panel should have no marks

  Scenario: help command opens help overlay
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "help"
    Then help should be visible

  Scenario: quit command sets should_quit
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "quit"
    Then the app should be quitting

  Scenario: exit command sets should_quit
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "exit"
    Then the app should be quitting

  Scenario: pwd shows current path in status
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "pwd"
    Then the status message should not be empty

  Scenario: empty command shows error
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command ""
    Then the status message should not be empty

  Scenario: Ctrl+Enter inserts filename into command line
    Given a source directory
    And a file "target.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "target.txt" in the left panel
    When I insert filename into command line
    Then the command line should contain "target.txt"

  Scenario: Ctrl+F inserts full path into command line
    Given a source directory
    And a file "full_path.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "full_path.txt" in the left panel
    When I insert full path into command line
    Then the command line should contain "full_path.txt"
