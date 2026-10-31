Feature: Provider token usage is refreshed without blocking the file panels
  Background:
    Given an isolated acceptance workspace

  Scenario: Menu reports combine provider pages and show cached input only once
    Given successful provider usage fixtures
    When I open token usage from the menu
    And the fixture usage response completes
    Then the rendered screen contains "Last 30 completed UTC days"
    And the token usage row for "OpenAI" contains "100 20 40 2"
    And the token usage row for "Anthropic" contains "190 50 40 3"
    And the token usage row for "OpenRouter" contains "100 50 n/a 2"
    And the rendered screen contains "Cached input is part of input"

  Scenario: F5 replaces the previous report with fresh provider totals
    Given successful provider usage fixtures
    When I open token usage from the menu
    And the fixture usage response completes
    And new provider usage is available
    And I dispatch the keys "F5"
    And the fixture usage response completes
    Then the token usage row for "OpenAI" contains "200 60 80 4"
    And the token usage row for "OpenRouter" contains "300 150 n/a 6"

  Scenario Outline: Provider failures are explained instead of displayed as zero usage
    Given provider usage fixtures fail with "<failure>"
    When I open token usage from the menu
    And the fixture usage response completes
    Then the token usage row for "OpenAI" reports "<message>" without numeric totals
    And the rendered screen contains "OPENAI_ADMIN_KEY"

    Examples:
      | failure       | message                   |
      | access denied | Access denied             |
      | missing key   | Not configured            |
      | rate limit    | Rate limited              |
      | network error | Network request failed    |

  Scenario: Closing a pending report keeps the file panels open after completion
    Given a delayed provider usage response
    When I open token usage from the menu
    And I start the fixture usage refresh
    And I dispatch the keys "Esc"
    And I release the prior usage response
    Then the token usage panel remains closed

  Scenario: An old response cannot populate a newly opened token usage panel
    Given a delayed provider usage response
    When I open token usage from the menu
    And I start the fixture usage refresh
    And I dispatch the keys "Esc"
    And I open token usage from the menu
    And I release the prior usage response
    Then the new token usage panel is still waiting for its own report
    When the fixture usage response completes
    Then the token usage row for "OpenAI" contains "200 60 80 4"
