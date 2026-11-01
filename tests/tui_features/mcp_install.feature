Feature: Install MCP server
  Scenario: The dialog opens from the Commands menu and registers in the current folder
    Given a TUI workspace with a fake MCP machine
    When I open the TUI at 80 by 24
    And I press "F9 Right Right e"
    Then the screen contains "Install MCP server"
    And the screen contains "(•) All projects"
    And the screen contains "[x] Claude Code"
    And the screen contains "{ Register }"
    When I press "Right"
    Then the screen contains "(•) Current folder"
    And the screen contains "Codex: reads .codex/config.toml only in folders it trusts."
    When I press "Enter"
    Then the screen contains "MCP server registration"
    And the screen contains "added"
    When I press "Esc"
    Then the screen does not contain "MCP server registration"
    And the screen does not contain "Install MCP server"
