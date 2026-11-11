# Purpose: BDD scenarios for the MCP server's stdio session lifecycle.
Feature: Server session lifecycle
  As an MCP client
  I want the server to shut itself down when its input stream ends
  So that a session leaves no orphaned process and no unflushed state

  Background:
    Given a temporary directory

  # This is the scenario that keeps src/main.rs measurable: coverage counters
  # are flushed by an atexit hook, so a server that only ever dies by signal
  # reports nothing. If the harness goes back to killing the child, or the
  # server stops honouring stdin EOF, this scenario turns red.
  Scenario: The server exits by itself once stdin reaches EOF
    Given a file "session.txt" with content "still here"
    When I get info for "session.txt"
    Then the result should contain "session.txt"
    And the server should exit cleanly after its input closes

  Scenario: A rejected request still ends in a clean shutdown
    When I try to get info for "no_such_file.txt"
    Then the operation should fail
    And the server should exit cleanly after its input closes
