Feature: Cloudflare DNS lists zones and records and requires confirmation before writing
  Background:
    Given an isolated acceptance workspace

  Scenario: Opening Cloudflare DNS loads zones and the selected zone's records
    Given successful Cloudflare DNS zone and record fixtures
    When I open Cloudflare DNS from the F9 menu
    And all Cloudflare DNS requests settle
    Then the rendered screen contains "Cloudflare DNS"
    And the rendered screen contains "example.com"
    And the rendered screen contains "example.org"
    And the rendered screen contains "Records of example.com"
    And the rendered screen contains "www.example.com"
    And 0 Cloudflare DNS writes were recorded

  Scenario: Moving to another zone loads that zone's records
    Given successful Cloudflare DNS zone and record fixtures
    When I open Cloudflare DNS from the F9 menu
    And all Cloudflare DNS requests settle
    And I dispatch the keys "Down"
    And all Cloudflare DNS requests settle
    Then the rendered screen contains "Records of example.org"
    And the rendered screen contains "api.example.org"

  Scenario: Editing a record asks for confirmation, supports Esc back, and writes on Enter
    Given successful Cloudflare DNS zone and record fixtures
    When I open Cloudflare DNS from the F9 menu
    And all Cloudflare DNS requests settle
    And I dispatch the keys "Tab Enter Backspace"
    And I type characters "5"
    And I dispatch the keys "Tab Tab Space Enter"
    Then the rendered screen contains "Confirm change"
    And the rendered screen contains "192.0.2.1 -> 192.0.2.5"
    And 0 Cloudflare DNS writes were recorded
    When I dispatch the keys "Esc"
    Then the rendered screen contains "Edit record"
    When I dispatch the keys "Enter Enter"
    And all Cloudflare DNS requests settle
    Then the rendered screen contains "Saved www.example.com"
    And 1 Cloudflare DNS writes were recorded

  Scenario: Pressing a looks up the current host and confirms a new A record
    Given successful Cloudflare DNS zone and record fixtures
    When I open Cloudflare DNS from the F9 menu
    And all Cloudflare DNS requests settle
    And I dispatch the keys "a"
    And all Cloudflare DNS requests settle
    Then the rendered screen contains "New record"
    And the rendered screen contains "host"
    And the rendered screen contains "198.51.100.42"
    When I dispatch the keys "Enter Enter"
    And all Cloudflare DNS requests settle
    Then 1 Cloudflare DNS writes were recorded
    And the rendered screen contains "Saved host.example.com"

  Scenario Outline: Cloudflare API errors keep the dialog open with clear status messages
    Given Cloudflare DNS requests fail with "<failure>"
    When I open Cloudflare DNS from the F9 menu
    And all Cloudflare DNS requests settle
    Then the rendered screen contains "<message>"
    And 0 Cloudflare DNS writes were recorded

    Examples:
      | failure       | message                                    |
      | missing token | set CLOUDFLARE_API_TOKEN                   |
      | rate limit    | Rate limited by Cloudflare; retry later    |
      | access denied | Access denied: check the API token's permissions |

  Scenario: Closing a pending Cloudflare DNS dialog discards delayed replies
    Given a delayed Cloudflare DNS response
    When I open Cloudflare DNS from the F9 menu
    And I start the Cloudflare DNS request
    And I dispatch the keys "Esc"
    And I release the delayed Cloudflare DNS response
    Then the overlay is closed

  Scenario: Reopening Cloudflare DNS ignores stale delayed replies from the closed dialog
    Given a delayed Cloudflare DNS response
    When I open Cloudflare DNS from the F9 menu
    And I start the Cloudflare DNS request
    And I dispatch the keys "Esc"
    And I open Cloudflare DNS from the F9 menu
    And I release the delayed Cloudflare DNS response
    Then the reopened Cloudflare DNS dialog still awaits its own response
    When all Cloudflare DNS requests settle
    Then the rendered screen contains "example.com"
