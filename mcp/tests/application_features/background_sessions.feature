Feature: Private background FFM sessions through MCP
  The MCP caller uses the real application while files and terminal streams remain reviewable.

  Scenario: A named editor save commits the edited bytes
    Given an isolated MCP workspace with an unchanged draft
    When the first session opens the draft in its editor
    And the first editor inserts "prefix " at the start
    Then the draft on disk contains "alpha" followed by a newline
    When the first editor invokes its named save action
    Then the draft on disk contains "prefix alpha" followed by a newline
    And the first editor displays "prefix alpha"

  Scenario: Unsaved editor buffers stay private to each background session
    Given an isolated MCP workspace with an unchanged draft
    And a second independent background FFM session
    When the first session opens the draft in its editor
    And the first editor inserts "private " at the start
    And the second session opens the draft in its editor
    Then the first editor displays "private alpha"
    And the second editor displays "alpha"
    And the second editor does not display "private alpha"
    And the draft on disk contains "alpha" followed by a newline

  Scenario: A private shell accepts input while state and cancellation remain available
    Given an isolated MCP workspace with an unchanged draft
    When the first session starts an interactive fixture shell
    Then the private shell output contains "ffm-ready"
    And MCP state remains responsive while that shell is running
    When I send fixture input to its private terminal
    Then the private shell output contains "reply:fixture-reply"
    When I cancel the first session's running job
    Then the original shell job finishes as cancelled before its final marker
