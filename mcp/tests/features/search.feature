# Purpose: BDD scenarios for search workflows.
Feature: Search and discovery via MCP tools
  As an AI assistant
  I want to search for files and get file info
  So that I can help the user find what they need

  Background:
    Given a temporary directory

  Scenario: Search finds matching files
    Given a file "report.txt" with content "data"
    And a file "report_v2.txt" with content "data"
    And a file "notes.txt" with content "other"
    When I search for "report"
    Then the search should return 2 results
    And the result should contain "report.txt"
    And the result should contain "report_v2.txt"

  Scenario: Search is case-insensitive
    Given a file "README.md" with content "hello"
    When I search for "readme"
    Then the search should return 1 results

  Scenario: Search respects max_results limit
    Given a file "match1.txt" with content "a"
    And a file "match2.txt" with content "b"
    And a file "match3.txt" with content "c"
    When I search for "match" with max results 2
    Then the search should return 2 results

  Scenario: Search files only excludes directories
    Given a file "data.log" with content "log"
    And a subdirectory "data_dir"
    When I search for "data" files only
    Then the search should return 1 results
    And the result should contain "data.log"

  Scenario: Search dirs only excludes files
    Given a file "data.log" with content "log"
    And a subdirectory "data_dir"
    When I search for "data" dirs only
    Then the search should return 1 results
    And the result should contain "data_dir"

  Scenario: Get file info returns metadata
    Given a file "info_test.txt" with content "test content"
    When I get info for "info_test.txt"
    Then the result should contain "file"
    And the result should contain "size"
    And the result should contain "txt"

  Scenario: List drives returns at least root
    When I list drives
    Then the result should contain "local"
