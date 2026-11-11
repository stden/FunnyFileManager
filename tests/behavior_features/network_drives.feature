Feature: Network drive snapshots stay in the user's private configuration
  Background:
    Given an isolated acceptance workspace
    And a private configuration with a saved server and custom settings

  Scenario: Discovered mounts preserve the existing server profiles and settings
    Given SMB and NFS mappings have been discovered
    When I save the detected network mappings
    Then the network mapping update succeeds
    And the private configuration contains 2 network mappings
    And drive Z points to "//files.example.test/team"
    And the saved server profiles and custom settings are unchanged

  Scenario: Offline discovery keeps previously saved network drives
    Given network mappings were previously saved
    When discovery returns no network mappings
    Then the network mapping update succeeds
    And the private configuration contains 2 network mappings
    And drive Z points to "//files.example.test/team"
    And the saved server profiles and custom settings are unchanged
    And the private configuration is unchanged byte for byte

  Scenario: A changed target replaces one drive without duplicating it
    Given network mappings were previously saved
    When drive Z is rediscovered at "//files.example.test/replacement"
    Then the network mapping update succeeds
    And the private configuration contains 2 network mappings
    And drive Z points to "//files.example.test/replacement"
    And the saved server profiles and custom settings are unchanged

  Scenario: Invalid YAML is never overwritten by drive discovery
    Given the private network configuration has invalid YAML
    And SMB and NFS mappings have been discovered
    When I save the detected network mappings
    Then the network mapping update is refused
    And the private configuration is unchanged byte for byte
