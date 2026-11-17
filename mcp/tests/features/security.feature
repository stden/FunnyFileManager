# Purpose: BDD scenarios for safe MCP server behavior.
Feature: Path security and validation
  As an MCP server
  I want to validate all paths
  So that malicious inputs cannot escape the filesystem

  Scenario: Empty path is rejected
    Given a temporary directory
    When I try to list ""
    Then the operation should fail with "empty"

  Scenario: Path with null byte is rejected
    Given a temporary directory
    When I try to list a path with null byte
    Then the operation should fail with "null byte"

  Scenario: Non-existent path is rejected for read operations
    Given a temporary directory
    When I try to get info for "nonexistent_file.txt"
    Then the operation should fail

  Scenario: Create file with non-existent deep parent
    Given a temporary directory
    When I create file "deep/nested/dir/file.txt" with content "deep"
    Then the file "deep/nested/dir/file.txt" should exist

  Scenario: Absolute path outside the allowed root is rejected
    Given a temporary directory
    When I try to list the absolute path "/etc"
    Then the operation should fail with "outside the allowed root"

  Scenario: Parent traversal cannot escape the allowed root
    Given a temporary directory
    When I try to list "../.."
    Then the operation should fail with "outside the allowed root"

  Scenario: Creating a file outside the allowed root is rejected
    Given a temporary directory
    When I try to create file "escape/../../../pwned.txt" with content "x"
    Then the operation should fail with "outside the allowed root"

  # The directory was confined but the file name was joined onto it raw, and
  # `join` throws the base away for an absolute operand. Answering at all was a
  # stat oracle: an existing target came back as "passwd_1", a missing one as
  # "passwd".
  Scenario: A unique name request may not traverse out of its directory
    Given a temporary directory
    When I ask for a unique name for "../../../../etc/passwd"
    Then the operation should fail with "plain file name"

  Scenario: A unique name request may not be an absolute path
    Given a temporary directory
    When I ask for a unique name for "/etc/passwd"
    Then the operation should fail with "plain file name"
