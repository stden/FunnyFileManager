# Purpose: BDD scenarios for content-level tools — append, grep, checksum, sizes.
Feature: Content tools via MCP tools
  As an AI assistant
  I want to append, grep and fingerprint file content
  So that I can inspect and extend files without rewriting them

  Background:
    Given a temporary directory

  # "first" must survive: a tool that opened the file for writing instead of
  # appending would leave only "second" behind.
  Scenario: Append adds to the end instead of overwriting
    Given a file "log.txt" with content "first"
    When I append "second" to file "log.txt"
    Then the file "log.txt" should contain "firstsecond"
    And the result field "bytes_added" should be 6

  Scenario: Append creates the file when it does not exist yet
    When I append "created by append" to file "fresh.txt"
    Then the file "fresh.txt" should exist
    And the file "fresh.txt" should contain "created by append"

  Scenario: Grep reports the matching line and its 1-based number
    Given a file "notes.txt" with content "alpha\nbeta\ngamma"
    When I grep for "beta"
    Then the search should return 1 results
    And match 1 should be "beta" at line 2

  Scenario: Grep matches regardless of case
    Given a file "notes.txt" with content "Needle in here"
    When I grep for "needle"
    Then the search should return 1 results
    And match 1 should be "Needle in here" at line 1

  Scenario: Grep restricted to one extension skips other files
    Given a file "keep.rs" with content "target"
    And a file "skip.txt" with content "target"
    When I grep for "target" in "rs" files
    Then the search should return 1 results
    And the result should contain "keep.rs"
    And the result should not contain "skip.txt"

  # The digest is the whole point of the tool: assert the exact SHA-256 of
  # "hello world" rather than merely that some hex string came back.
  Scenario: Checksum returns the SHA-256 of the file bytes
    Given a file "payload.bin" with content "hello world"
    When I checksum "payload.bin"
    Then the result field "sha256" should equal "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    And the result field "size" should be 11
    And the result field "size_human" should equal "11 B"

  Scenario: Checksum of a directory is rejected
    Given a subdirectory "not_a_file"
    When I try to checksum "not_a_file"
    Then the operation should fail with "not a file"

  Scenario: Sizes below a kilobyte stay in bytes
    When I format 1023 bytes
    Then the result field "formatted" should equal "1023 B"

  Scenario: A kilobyte and a half formats with two decimals
    When I format 1536 bytes
    Then the result field "formatted" should equal "1.50 KB"

  Scenario: A megabyte switches unit at the 1024 KB boundary
    When I format 1048576 bytes
    Then the result field "formatted" should equal "1.00 MB"

  Scenario: A unique name is the requested one when nothing collides
    When I ask for a unique name for "report.txt"
    Then the result field "unique_name" should equal "report.txt"

  Scenario: A unique name gains a counter before the extension
    Given a file "report.txt" with content "first"
    And a file "report_1.txt" with content "second"
    When I ask for a unique name for "report.txt"
    Then the result field "unique_name" should equal "report_2.txt"
