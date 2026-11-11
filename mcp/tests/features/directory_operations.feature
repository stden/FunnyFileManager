# Purpose: BDD scenarios for directory creation and traversal operations.
Feature: Directory operations via MCP tools
  As an AI assistant
  I want to create, list, and navigate directories
  So that I can organize files for the user

  Background:
    Given a temporary directory

  Scenario: Create a nested directory
    When I create directory "a/b/c"
    Then the directory "a/b/c" should exist

  Scenario: List directory shows files and subdirectories
    Given a file "alpha.txt" with content "a"
    And a file "beta.txt" with content "b"
    And a subdirectory "gamma"
    When I list the temporary directory
    Then the result should contain "alpha.txt"
    And the result should contain "beta.txt"
    And the result should contain "gamma"

  Scenario: List directory sorts directories before files
    Given a file "zebra.txt" with content "z"
    And a subdirectory "aardvark"
    When I list the temporary directory
    Then "aardvark" should appear before "zebra.txt" in entries

  Scenario: List a file path fails
    Given a file "not_a_dir.txt" with content "x"
    When I try to list "not_a_dir.txt"
    Then the operation should fail with "not a directory"

  Scenario: Disk usage calculates total size
    Given a file "a.txt" with content "hello"
    And a file "b.txt" with content "world"
    When I get disk usage of the temporary directory
    Then the size should be 10

  Scenario: Tree shows directory structure
    Given a subdirectory "src"
    And a file "src/main.rs" with content "fn main() {}"
    And a file "README.md" with content "# Hello"
    When I get tree of the temporary directory with depth 2
    Then the result should contain "src/"
    And the result should contain "main.rs"
    And the result should contain "README.md"
