# Purpose: BDD scenarios for copy and move operations.
Feature: Copy and move operations via MCP tools
  As an AI assistant
  I want to copy and move files and directories
  So that I can reorganize the user's file system

  Background:
    Given a temporary directory

  Scenario: Copy a file
    Given a file "original.txt" with content "copy me"
    When I copy "original.txt" to "duplicate.txt"
    Then the file "original.txt" should exist
    And the file "duplicate.txt" should exist
    And the file "duplicate.txt" should contain "copy me"

  Scenario: Copy a directory recursively
    Given a subdirectory "project"
    And a file "project/main.rs" with content "fn main() {}"
    And a file "project/lib.rs" with content "pub mod lib;"
    When I copy directory "project" to "backup"
    Then the file "backup/main.rs" should exist
    And the file "backup/lib.rs" should exist
    And the file "backup/main.rs" should contain "fn main() {}"

  Scenario: Move renames a file
    Given a file "old_name.txt" with content "rename me"
    When I move "old_name.txt" to "new_name.txt"
    Then the file "old_name.txt" should not exist
    And the file "new_name.txt" should exist
    And the file "new_name.txt" should contain "rename me"

  Scenario: Copy non-existent source fails
    When I try to copy "missing.txt" to "dest.txt"
    Then the operation should fail with "source not found"

  Scenario: Move non-existent source fails
    When I try to move "missing.txt" to "dest.txt"
    Then the operation should fail with "source not found"

  Scenario: Copy directory into itself fails
    Given a subdirectory "project"
    When I try to copy "project" to "project/nested"
    Then the operation should fail with "cannot copy a directory into itself"

  Scenario: Move directory into itself fails
    Given a subdirectory "project"
    When I try to move "project" to "project/nested"
    Then the operation should fail with "cannot copy a directory into itself"
