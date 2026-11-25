Feature: Jira issues browser filters, views details, edits JQL, and exports markdown
  Background:
    Given an isolated acceptance workspace

  Scenario: Opening Jira issues from F9 loads issues from search and user identity
    Given configured Jira issue fixtures
    When I open Jira issues from the F9 menu
    And the Jira request completes
    Then the rendered screen contains "Jira Issues"
    And the rendered screen contains "DEMO-1"
    And the rendered screen contains "Fix crash"
    And the rendered screen contains "DEMO-2"
    And the rendered screen contains "Add export"

  Scenario: Typing filters issues, Esc clears filter, and Enter opens detail reader
    Given configured Jira issue fixtures
    When I open Jira issues from the F9 menu
    And the Jira request completes
    And I type characters "crash"
    Then the rendered screen contains "DEMO-1"
    And the rendered screen excludes "DEMO-2"
    When I dispatch the keys "Esc"
    Then the rendered screen contains "DEMO-2"
    When I dispatch the keys "Enter"
    Then the rendered screen contains "DEMO-1 — Open"
    And the rendered screen contains "Repro steps here"
    When I dispatch the keys "Esc"
    Then the rendered screen contains "Jira Issues"

  Scenario: Editing JQL via F2 requeries Jira and Ctrl+R reloads the current query
    Given configured Jira issue fixtures
    When I open Jira issues from the F9 menu
    And the Jira request completes
    And I dispatch the keys "F2 Ctrl+A Backspace"
    Then the rendered screen contains "Edit Jira JQL"
    When I type characters "project = DEMO"
    And I dispatch the keys "Enter"
    And the Jira request completes
    Then the rendered screen contains "JQL: project = DEMO"
    And Jira queries recorded JQL "project = DEMO"
    When I dispatch the keys "Space"
    Then the rendered screen contains "Marks: 1"
    When I dispatch the keys "Ctrl+R"
    And the Jira request completes
    Then the rendered screen contains "Loaded"

  Scenario: F5 exports marked tickets and F7 exports a snapshot folder
    Given configured Jira issue fixtures
    When I open Jira issues from the F9 menu
    And the Jira request completes
    And I dispatch the keys "Space F5"
    And the Jira request completes
    Then the active panel contains "DEMO-1/issue.md" with "Fix crash"
    When I dispatch the keys "F7"
    And the Jira request completes
    Then a Jira snapshot folder exists in the active panel

  Scenario Outline: Jira errors report clear status without closing the dialog
    Given Jira fixtures fail with "<failure>"
    When I open Jira issues from the F9 menu
    And the Jira request completes
    Then the rendered screen contains "<message>"

    Examples:
      | failure      | message                        |
      | unauthorized | Jira HTTP 401: invalid token   |
      | invalid json | Unexpected Jira search response: missing issues list |

  Scenario: Live mode without Jira credentials opens setup instructions without crashing
    Given Jira access is set to network without credentials
    When I open Jira issues from the F9 menu
    Then the rendered screen contains "Set JIRA_PERSONAL_TOKEN / JIRA_URL or press F4 to edit ffm.yml"

  Scenario: Offline Jira access fails cleanly without network calls
    Given configured Jira issue fixtures
    When I open Jira issues from the F9 menu
    And the live offline Jira worker pumps
    Then the rendered screen contains "Unavailable in replay or isolated sessions"

  Scenario: Reopening Jira discards delayed fetch replies from a closed dialog
    Given a delayed Jira response
    When I open Jira issues from the F9 menu
    And I start the Jira request
    And I dispatch the keys "Esc"
    And I open Jira issues from the F9 menu
    And I release the delayed Jira response
    Then the reopened Jira dialog still awaits its own response
    When the Jira request completes
    Then the rendered screen contains "DEMO-1"
