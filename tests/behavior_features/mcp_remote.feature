Feature: Remote HTTP MCP services discover endpoints, list tools, validate arguments, and invoke calls
  Background:
    Given an isolated acceptance workspace

  Scenario: Opening MCP services discovers .mcp.json servers and hides disabled tools
    Given an HTTP MCP server "alpha" configured in the active folder with disabled tool "hidden_tool"
    When I open MCP services from the F9 menu
    And the MCP remote request completes
    Then the rendered screen contains "MCP Services (HTTP)"
    And the rendered screen contains "alpha"
    And the rendered screen contains "search"
    And the rendered screen excludes "hidden_tool"

  Scenario: Selecting a tool, calling with JSON arguments, and saving output to disk
    Given an HTTP MCP server "alpha" configured in the active folder with disabled tool "hidden_tool"
    When I open MCP services from the F9 menu
    And the MCP remote request completes
    And I dispatch the keys "Tab"
    Then the rendered screen contains "Focus: Servers"
    When I dispatch the keys "Tab Enter"
    Then the rendered screen contains "Invoke MCP Tool"
    And the rendered screen contains "Call tool search"
    When I dispatch the keys "Enter"
    And the MCP remote request completes
    Then the rendered screen contains "matched 3 files"
    When I dispatch the keys "F2"
    Then the active panel contains "mcp-output.txt" with "matched 3 files"
    When I dispatch the keys "Esc"
    Then the rendered screen contains "MCP Services (HTTP)"

  Scenario: Invalid JSON arguments keep CallInput open with an inline error notice
    Given an HTTP MCP server "alpha" configured in the active folder with disabled tool "hidden_tool"
    When I open MCP services from the F9 menu
    And the MCP remote request completes
    And I dispatch the keys "Enter Ctrl+A Backspace"
    And I type characters "{bad"
    And I dispatch the keys "Enter"
    Then the rendered screen contains "Invalid JSON arguments"
    And the rendered screen contains "Invoke MCP Tool"
    And the MCP transport recorded 3 JSON-RPC requests
    When I dispatch the keys "Esc"
    Then the rendered screen contains "MCP Services (HTTP)"

  Scenario: Empty folder shows discovery guidance and RPC HTTP errors surface in status
    When I open MCP services from the F9 menu
    Then the rendered screen contains "No HTTP MCP services found"
    When I dispatch the keys "Esc"
    Given an HTTP MCP server returning HTTP 403 error
    When I open MCP services from the F9 menu
    And the MCP remote request completes
    Then the rendered screen contains "HTTP 403"

  Scenario: Offline MCP access reports disabled status without reaching the network
    Given an HTTP MCP server "alpha" configured in the active folder with disabled tool "hidden_tool"
    When I open MCP services from the F9 menu
    And the live offline MCP worker pumps
    Then the rendered screen contains "Unavailable in replay or isolated sessions"

  Scenario: Reopening MCP services discards delayed tools replies from a closed dialog
    Given a delayed HTTP MCP server response
    When I open MCP services from the F9 menu
    And I start the MCP remote worker
    And I dispatch the keys "Esc"
    And I open MCP services from the F9 menu
    And I release the delayed MCP remote response
    Then the reopened MCP dialog still awaits its own tools reply
    When the MCP remote request completes
    Then the rendered screen contains "search"
