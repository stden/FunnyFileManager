# Purpose: BDD scenarios for the sort_directory listing modes.
Feature: Sorted directory listings via MCP tools
  As an AI assistant
  I want listings ordered the way I asked for
  So that the first entry I read is the one the sort promised

  Background:
    Given a temporary directory

  Scenario: Sorting by name puts directories first, then names ascending
    Given a file "b.txt" with content "b"
    And a file "a.txt" with content "a"
    And a subdirectory "zdir"
    When I sort the temporary directory by "name"
    Then the "entries" entries should be named "zdir, a.txt, b.txt"

  Scenario: Sorting by extension groups extensions and breaks ties by name
    Given a file "second.rs" with content "s"
    And a file "first.rs" with content "f"
    And a file "notes.txt" with content "n"
    When I sort the temporary directory by "extension"
    Then the "entries" entries should be named "first.rs, second.rs, notes.txt"

  Scenario: Sorting by size lists the largest file first
    Given a file "small.txt" with content "s"
    And a file "medium.txt" with content "mmm"
    And a file "large.txt" with content "lllllll"
    When I sort the temporary directory by "size"
    Then the "entries" entries should be named "large.txt, medium.txt, small.txt"

  Scenario: An unknown sort mode falls back to sorting by name
    Given a file "b.txt" with content "b"
    And a file "a.txt" with content "a"
    When I sort the temporary directory by "nonsense"
    Then the "entries" entries should be named "a.txt, b.txt"
    And the result field "sort" should equal "nonsense"

  Scenario: Sorting a file path fails
    Given a file "plain.txt" with content "x"
    When I sort the file "plain.txt" by "name"
    Then the operation should fail with "not a directory"
