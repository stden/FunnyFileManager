Feature: Server profiles are saved in the private user YAML without losing other settings
  Background:
    Given an isolated acceptance workspace
    And a private ffm.yml for server profiles

  Scenario: Legacy server profiles migrate to YAML while the JSON file remains intact
    Given a legacy server profile "legacy.example.test"
    When I open the user server profile panel
    Then the rendered screen contains "legacy.example.test"
    When I create saved server profile "added.example.test"
    And I reload the user server profiles
    Then server profile "legacy.example.test" is saved with port 22
    And server profile "added.example.test" is saved with port 22
    And the original legacy server profile file is unchanged
    And saved profiles describe authentication without credential fields

  Scenario: Creating editing and deleting a profile survives reloads and preserves unrelated settings
    Given the private profile configuration contains network drives and custom options
    When I open the user server profile panel
    And I create saved server profile "work.example.test"
    And I reload the user server profiles
    Then the rendered screen contains "work.example.test"
    When I change server profile "work.example.test" to port 2222
    And I reload the user server profiles
    Then server profile "work.example.test" is saved with port 2222
    And network drive mappings and custom profile options remain unchanged
    When I delete saved server profile "work.example.test"
    And I reload the user server profiles
    Then server profile "work.example.test" is absent after reload
    And network drive mappings and custom profile options remain unchanged

  Scenario: A concurrent profile save is preserved when an older session tries to write
    Given the private profile configuration contains network drives and custom options
    When I open the user server profile panel
    And another session saves server profile "newer.example.test"
    And I attempt to create saved server profile "stale.example.test"
    Then the rejected profile save preserves the existing configuration bytes
    And the profile dialog remains open for correction
    When I reload the user server profiles
    Then server profile "newer.example.test" is saved with port 22
    And server profile "stale.example.test" is absent after reload
    And network drive mappings and custom profile options remain unchanged

  Scenario Outline: Unusable profile documents are never overwritten by an attempted edit
    Given a <damage> server profile document
    When I open the user server profile panel
    And I attempt to create saved server profile "attempted.example.test"
    Then the rejected profile save preserves the existing configuration bytes
    And the profile dialog remains open for correction

    Examples:
      | damage    |
      | malformed |
      | duplicate |
