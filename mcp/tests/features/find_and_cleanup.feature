# Purpose: BDD scenarios for the filtered find and pattern-delete tools.
Feature: Filtered find and pattern cleanup via MCP tools
  As an AI assistant
  I want filters that exclude exactly what they claim to
  So that a cleanup never reaches a file the user wanted to keep

  Background:
    Given a temporary directory

  Scenario: Find by name glob crosses directories
    Given a file "root.log" with content "r"
    And a file "nested/inner.log" with content "i"
    And a file "nested/notes.txt" with content "n"
    When I find entries named "*.log"
    Then the search should return 2 results
    And the "results" entries should be named "root.log, inner.log" in any order

  Scenario: Find by type returns directories only
    Given a file "file.txt" with content "f"
    And a subdirectory "dir_one"
    And a subdirectory "dir_two"
    When I find entries of type "dir"
    Then the search should return 2 results
    And the "results" entries should be named "dir_one, dir_two" in any order

  @unix
  Scenario: Find by type tells a symlink apart from its target
    Given a file "real.txt" with content "data"
    And a symlink "alias.txt" pointing to "real.txt"
    When I find entries of type "symlink"
    Then the search should return 1 results
    And the "results" entries should be named "alias.txt"

  # 4 bytes is exactly the minimum, so it must be included — an off-by-one in
  # the size filter drops it.
  Scenario: Minimum size includes files exactly on the boundary
    Given a file "tiny.txt" with content "abc"
    And a file "exact.txt" with content "abcd"
    And a file "large.txt" with content "abcdefgh"
    When I find entries of at least 4 bytes
    Then the search should return 2 results
    And the "results" entries should be named "exact.txt, large.txt" in any order

  # depth 1 admits the immediate subdirectory's files but stops before the
  # level below it.
  Scenario: Max depth stops the walk one level down
    Given a file "top.txt" with content "t"
    And a file "one/mid.txt" with content "m"
    And a file "one/two/deep.txt" with content "d"
    When I find entries no deeper than 1
    Then the search should return 2 results
    And the "results" entries should be named "top.txt, mid.txt" in any order

  Scenario: Deleting by pattern reaches into subdirectories
    Given a file "scratch.tmp" with content "abc"
    And a file "sub/other.tmp" with content "abc"
    And a file "keep.txt" with content "keep me"
    When I delete files matching "*.tmp"
    Then the result field "deleted_count" should be 2
    And the result field "total_size" should be 6
    And the file "scratch.tmp" should not exist
    And the file "sub/other.tmp" should not exist
    And the file "keep.txt" should exist

  Scenario: A dry run reports what it would delete without deleting it
    Given a file "scratch.tmp" with content "abc"
    And a file "keep.txt" with content "keep me"
    When I preview deleting files matching "*.tmp"
    Then the result field "deleted_count" should be 1
    And the file "scratch.tmp" should exist
    And the file "keep.txt" should exist

  Scenario: A dry run replace counts matches but leaves the file alone
    Given a file "doc.txt" with content "old and old again"
    When I preview replacing "old" with "new"
    Then the result field "total_replacements" should be 2
    And the result field "files_affected" should be 1
    And the file "doc.txt" should contain "old and old again"
    And the file "doc.txt" should not contain "new"
