# Purpose: BDD scenarios for F9 Commands > Install MCP server.
Feature: Install the MCP server for AI clients
  As a user of AI coding clients
  I want FFM to register ffm-mcp with them
  So that they can use FFM's file tools without hand-editing their configs

  Background:
    Given a fake machine with Claude Code, Codex, Junie and Kiro detected
    And ffm-mcp next to ffm

  Scenario: Registering for all projects writes the user-level configs
    When I register the MCP server for all projects
    Then the claude CLI ran "mcp add --scope user --transport stdio ffm-mcp --" with the server
    And the user-level Codex, Junie and Kiro configs register ffm-mcp without FFM_MCP_ROOT

  Scenario: Registering for the current folder roots every entry there
    When I register the MCP server for the current folder
    Then the folder's .mcp.json, .codex/config.toml, .junie/mcp/mcp.json and .kiro/settings/mcp.json register ffm-mcp with FFM_MCP_ROOT set to the folder
    And the claude CLI was not run

  Scenario: A config that does not parse is left untouched
    Given the user-level Kiro config contains "{ broken"
    When I register the MCP server for all projects
    Then the user-level Kiro config still contains exactly "{ broken"
    And the user-level Junie config registers ffm-mcp without FFM_MCP_ROOT

  Scenario: Other servers and keys survive, and a second run changes nothing
    Given the user-level Junie config holds another server "other" and a key "theme"
    When I register the MCP server for all projects
    And I register the MCP server for all projects
    Then the user-level Junie config still holds the server "other" and the key "theme"
    And the user-level Junie config was written once

  Scenario: An entry for another binary is updated in place
    Given the user-level Codex config registers ffm-mcp at "/old/ffm-mcp" with startup_timeout_sec 20
    When I register the MCP server for all projects
    Then the user-level Codex config registers ffm-mcp at the server with startup_timeout_sec 20

  Scenario: A missing ffm-mcp is explained and nothing is written
    Given no ffm-mcp next to ffm or on PATH
    When I register the MCP server for all projects
    Then no config file was created or changed
    And the claude CLI was not run

  Scenario: A claude CLI that times out fails only Claude Code
    Given the claude CLI times out
    When I register the MCP server for all projects
    Then the user-level Codex, Junie and Kiro configs register ffm-mcp without FFM_MCP_ROOT

  Scenario: The home folder as the current folder leaves the user-level configs alone
    Given the panels show the home folder
    When I register the MCP server for the current folder
    Then no user-level Codex, Junie or Kiro config was written
    And the claude CLI was not run
