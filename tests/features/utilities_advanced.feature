# Purpose: BDD scenarios for utility commands and support flows.
Feature: Advanced utility function scenarios
  As a file manager developer
  I want utility functions to handle all edge cases
  So that the application is robust

  Scenario: Format zero bytes
    When I format size 0
    Then the formatted size should be "0 B"

  Scenario: Format 1 byte
    When I format size 1
    Then the formatted size should be "1 B"

  Scenario: Format 1023 bytes stays in bytes
    When I format size 1023
    Then the formatted size should be "1023 B"

  Scenario: Format exactly 1 KB
    When I format size 1024
    Then the formatted size should be "1.00 KB"

  Scenario: Format 1.5 KB
    When I format size 1536
    Then the formatted size should be "1.50 KB"

  Scenario: Format exactly 1 MB
    When I format size 1048576
    Then the formatted size should be "1.00 MB"

  Scenario: Format 2.5 MB
    When I format size 2621440
    Then the formatted size should be "2.50 MB"

  Scenario: Format exactly 1 GB
    When I format size 1073741824
    Then the formatted size should be "1.00 GB"

  Scenario: Format 10 GB
    When I format size 10737418240
    Then the formatted size should be "10.00 GB"

  Scenario: Format large file 999 bytes
    When I format size 999
    Then the formatted size should be "999 B"

  Scenario: Unique name when file does not exist
    Given a temporary directory
    When I request unique name for "fresh.txt"
    Then the unique name should be "fresh.txt"

  Scenario: Unique name when file already exists
    Given a temporary directory
    And a file "taken.txt" with content "x"
    When I request unique name for "taken.txt"
    Then the unique name should be "taken_1.txt"

  Scenario: Unique name with multiple conflicts
    Given a temporary directory
    And a file "dup.txt" with content "x"
    And a file "dup_1.txt" with content "x"
    And a file "dup_2.txt" with content "x"
    When I request unique name for "dup.txt"
    Then the unique name should be "dup_3.txt"

  Scenario: Copy directory recursively preserves structure
    Given a temporary directory
    And a subdirectory "src_tree" with file "root.txt"
    When I copy directory "src_tree" to "dst_tree"
    Then "dst_tree" should exist in the temporary directory
    And "dst_tree/root.txt" should exist in the temporary directory

  Scenario: Refresh panels reloads both panels
    Given a source directory
    And a file "before.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I refresh panels
    Then the status message should contain "refreshed"
    And the left panel should have entries
