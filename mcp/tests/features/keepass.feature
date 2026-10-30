# Purpose: Verifies KeePass export through the real MCP stdio transport.
Feature: KeePass export via the kbdx MCP tool
  As an MCP client
  I want a KeePass vault rendered as structured text
  So that all decrypted data can be consumed without writing a plaintext file

  Background:
    Given a temporary directory
    And a KeePass database "vault.kdbx" protected by "bdd-password"

  Scenario: JSON is the default and includes protected and custom fields
    When I export KeePass database "vault.kdbx" with password "bdd-password"
    Then the result should contain "BDD Vault"
    And the result should contain "Deploy"
    And the result should contain "bdd-secret"
    And the result should contain "custom-value"

  Scenario: YAML carries the same decrypted fields
    When I export KeePass database "vault.kdbx" with password "bdd-password" as "yaml"
    Then the result should contain "database_name: BDD Vault"
    And the result should contain "bdd-secret"
    And the result should contain "custom-value"

  Scenario: Dotenv includes passwords but omits other secret fields
    When I export KeePass database "vault.kdbx" with password "bdd-password" as "env"
    Then the result should contain "KDBX_DEPLOY_PASSWORD"
    And the result should contain "bdd-secret"
    And the result should not contain "custom-value"
