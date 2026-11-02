# Purpose: BDD scenarios for file operation behavior.
Feature: File operations via MCP tools
  As an AI assistant
  I want to create, read, write, and delete files
  So that I can manage files on behalf of the user

  Background:
    Given a temporary directory

  Scenario: Create a new file with content
    When I create file "hello.txt" with content "Hello, World!"
    Then the file "hello.txt" should exist
    And the file "hello.txt" should contain "Hello, World!"

  Scenario: Create file fails if it already exists
    Given a file "exists.txt" with content "original"
    When I try to create file "exists.txt" with content "duplicate"
    Then the operation should fail with "already exists"

  Scenario: Write overwrites existing file
    Given a file "data.txt" with content "old content"
    When I write "new content" to file "data.txt"
    Then the file "data.txt" should contain "new content"

  Scenario: View a text file
    Given a file "readme.txt" with content "line1\nline2\nline3"
    When I view file "readme.txt"
    Then the result should contain "total_lines"
    And the result should contain "line1"

  Scenario: View file with offset and limit
    Given a file "lines.txt" with content "aaa\nbbb\nccc\nddd"
    When I view file "lines.txt" with offset 1 and limit 2
    Then the result should contain "bbb"
    And the result should contain "ccc"
    And the result should not contain "aaa"

  Scenario: View a directory should fail
    When I try to view the temporary directory
    Then the operation should fail with "cannot view a directory"

  Scenario: Delete a file
    Given a file "trash.txt" with content "delete me"
    When I delete "trash.txt"
    Then the file "trash.txt" should not exist

  Scenario: Delete non-existent path fails
    When I try to delete "ghost.txt"
    Then the operation should fail with "not found"
