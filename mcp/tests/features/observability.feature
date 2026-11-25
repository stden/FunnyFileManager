# Purpose: BDD scenarios for MCP observability and status reporting.
Feature: Tree depth, pagination, and size reporting via MCP tools
  As an AI assistant
  I want stable introspection helpers
  So that I can inspect project files safely and predictably

  Background:
    Given a temporary directory

  Scenario: Tree depth limits nested output
    Given a subdirectory "src/deep"
    And a file "src/deep/hidden.txt" with content "secret"
    And a file "src/lib.rs" with content "pub fn hello() {}"
    When I get tree of the temporary directory with depth 1
    Then the result should contain "src/"
    And the result should not contain "hidden.txt"

  Scenario: Disk usage includes nested files
    Given a file "root.txt" with content "1234"
    And a file "nested/data.txt" with content "567890"
    When I get disk usage of the temporary directory
    Then the size should be 10

  Scenario: Viewing with offset near end returns remaining lines only
    Given a file "tail.txt" with content "line1\nline2\nline3\nline4"
    When I view file "tail.txt" with offset 2 and limit 10
    Then the result should contain "line3"
    And the result should contain "line4"
    And the result should not contain "line1"
    And the result should not contain "line2"
