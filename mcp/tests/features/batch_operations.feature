# Purpose: BDD scenarios for the batch copy/move/delete tools.
Feature: Batch operations via MCP tools
  As an AI assistant
  I want to act on several paths in one call
  So that bulk reorganisation does not need one round trip per file

  Background:
    Given a temporary directory

  Scenario: Batch copy leaves the sources in place
    Given a file "one.txt" with content "1"
    And a file "two.txt" with content "2"
    When I batch copy "one.txt, two.txt" into "archive"
    Then the result list "copied" should be "one.txt, two.txt"
    And the result field "total_errors" should be 0
    And the file "archive/one.txt" should contain "1"
    And the file "archive/two.txt" should contain "2"
    And the file "one.txt" should exist
    And the file "two.txt" should exist

  Scenario: Batch copy takes a directory with it
    Given a subdirectory "pkg"
    And a file "pkg/inner.txt" with content "nested"
    When I batch copy "pkg" into "archive"
    Then the result field "total_copied" should be 1
    And the file "archive/pkg/inner.txt" should contain "nested"

  Scenario: Batch move removes the sources
    Given a file "one.txt" with content "1"
    And a file "two.txt" with content "2"
    When I batch move "one.txt, two.txt" into "moved"
    Then the result list "moved" should be "one.txt, two.txt"
    And the file "moved/one.txt" should contain "1"
    And the file "one.txt" should not exist
    And the file "two.txt" should not exist

  # A batch must not abort on the first bad entry: the good paths still get
  # processed and the failure is reported alongside them.
  Scenario: Batch copy reports per-path failures and keeps going
    Given a file "good.txt" with content "ok"
    When I batch copy "good.txt, missing.txt" into "archive"
    Then the result field "total_copied" should be 1
    And the result field "total_errors" should be 1
    And the result list "copied" should be "good.txt"
    And the file "archive/good.txt" should exist

  Scenario: Batch delete removes every listed path
    Given a file "a.txt" with content "a"
    And a file "b.txt" with content "b"
    And a subdirectory "c"
    When I batch delete "a.txt, b.txt, c"
    Then the result field "total_deleted" should be 3
    And the result field "total_errors" should be 0
    And the file "a.txt" should not exist
    And the file "b.txt" should not exist
    And the file "c" should not exist

  Scenario: Batch delete of an unknown path is reported, not fatal
    Given a file "a.txt" with content "a"
    When I batch delete "a.txt, ghost.txt"
    Then the result field "total_deleted" should be 1
    And the result field "total_errors" should be 1
    And the file "a.txt" should not exist
