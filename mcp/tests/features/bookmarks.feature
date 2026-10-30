# Purpose: BDD scenarios for the named-bookmark tools.
Feature: Bookmarks via MCP tools
  As an AI assistant
  I want to remember directories by name
  So that I can jump back to them in a later call

  # HOME is redirected to the scenario's temp directory, so every bookmark
  # store below is created fresh and never touches the real ~/.ffm.
  Background:
    Given a temporary directory

  Scenario: A bookmark survives into the next call
    When I bookmark the temporary directory as "work"
    And I look up the bookmark "work"
    Then the result field "bookmark" should equal "work"
    And the result field "path" should be the temporary directory

  Scenario: Looking up an unknown bookmark fails
    When I look up the bookmark "never_set"
    Then the operation should fail with "bookmark not found"

  Scenario: Listing counts every saved bookmark
    When I bookmark the temporary directory as "first"
    And I bookmark the temporary directory as "second"
    And I list bookmarks
    Then the result field "count" should be 2
    And the "bookmarks" entries should be named "first, second"

  Scenario: A removed bookmark can no longer be looked up
    When I bookmark the temporary directory as "temporary"
    And I remove the bookmark "temporary"
    Then the result field "deleted_bookmark" should equal "temporary"
    When I look up the bookmark "temporary"
    Then the operation should fail with "bookmark not found"

  Scenario: Removing an unknown bookmark fails
    When I remove the bookmark "never_set"
    Then the operation should fail with "bookmark not found"

  Scenario: Overwriting an out-of-root bookmark entry is rejected
    Given a file ".ffm/bookmarks.json" with content '{"escape": "/etc"}'
    When I bookmark the temporary directory as "escape"
    Then the operation should fail with "outside the allowed root"

  Scenario: Looking up an out-of-root bookmark entry is rejected
    Given a file ".ffm/bookmarks.json" with content '{"escape": "/etc"}'
    When I look up the bookmark "escape"
    Then the operation should fail with "outside the allowed root"

  Scenario: Removing an out-of-root bookmark entry is rejected
    Given a file ".ffm/bookmarks.json" with content '{"escape": "/etc"}'
    When I remove the bookmark "escape"
    Then the operation should fail with "outside the allowed root"

  Scenario: Listing hides bookmarks that point outside the allowed root
    Given a subdirectory "keep"
    And a file ".ffm/bookmarks.json" with content '{"keep": "keep", "escape": "/etc"}'
    When I list bookmarks
    Then the result field "count" should be 1
    And the "bookmarks" entries should be named "keep"
