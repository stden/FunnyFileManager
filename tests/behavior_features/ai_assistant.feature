Feature: AI assistant queries providers, extracts commands, and saves transcripts
  Background:
    Given an isolated acceptance workspace

  Scenario: Opening AI assistant and switching between providers
    Given configured AI assistant fixtures
    When I open AI assistant from the F9 menu
    Then the rendered screen contains "AI Assistant"
    And the rendered screen contains "[Anthropic]"
    When I dispatch the keys "Tab"
    Then the rendered screen contains "[OpenAI]"
    When I dispatch the keys "Alt+3"
    Then the rendered screen contains "[Codex]"
    When I dispatch the keys "Alt+1"
    Then the rendered screen contains "[Anthropic]"

  Scenario Outline: All three AI provider formats decode replies and shell commands
    Given configured AI assistant fixtures
    When I open AI assistant from the F9 menu
    And I select AI provider "<key>"
    And I type characters "list rust files"
    And I dispatch the keys "Enter"
    And the AI request completes
    Then the rendered screen contains "[You]"
    And the rendered screen contains "[AI]"
    And the rendered screen contains "Cmd (F5/Ctrl+X): <cmd>"
    And the AI transport recorded endpoint "<endpoint>"

    Examples:
      | key   | cmd                   | endpoint             |
      | Alt+1 | ls -la                | /v1/messages         |
      | Alt+2 | rg --files            | /v1/chat/completions |
      | Alt+3 | find . -name '*.rs'   | /v1/responses        |

  Scenario: Pressing F5 places the extracted command into the command line
    Given configured AI assistant fixtures
    When I open AI assistant from the F9 menu
    And I type characters "show files"
    And I dispatch the keys "Enter"
    And the AI request completes
    And I dispatch the keys "F5"
    Then the command line contains "ls -la"
    And the rendered screen contains "ls -la"

  Scenario: F2 saves the AI transcript to disk and F8 clears conversation history
    Given configured AI assistant fixtures
    When I open AI assistant from the F9 menu
    And I type characters "summarize build"
    And I dispatch the keys "Enter"
    And the AI request completes
    And I dispatch the keys "F2"
    Then the active panel contains "ai-transcript.md" with "### You"
    When I dispatch the keys "F8"
    Then the rendered screen excludes "[AI]"
    And the rendered screen contains "Ask about files, commands, or errors"

  Scenario Outline: AI failures surface clear error status messages
    Given AI assistant fixtures fail with "<failure>"
    When I open AI assistant from the F9 menu
    And I type characters "help"
    And I dispatch the keys "Enter"
    And the AI request completes
    Then the rendered screen contains "<message>"

    Examples:
      | failure          | message                                        |
      | missing key      | ANTHROPIC_API_KEY                              |
      | rate limit       | AI HTTP 429: too many requests                 |
      | invalid response | Anthropic response missing content blocks      |

  Scenario: Offline AI access reports disabled status without reaching any transport
    Given configured AI assistant fixtures
    When I open AI assistant from the F9 menu
    And I type characters "hello"
    And I dispatch the keys "Enter"
    And the live offline AI worker pumps
    Then the rendered screen contains "Unavailable in replay or isolated sessions"

  Scenario: Reopening AI assistant discards delayed replies from a closed dialog
    Given a delayed AI assistant response
    When I open AI assistant from the F9 menu
    And I type characters "first"
    And I dispatch the keys "Enter"
    And I start the AI worker
    And I dispatch the keys "Esc"
    And I open AI assistant from the F9 menu
    And I release the delayed AI response
    Then the reopened AI dialog remains idle and empty
