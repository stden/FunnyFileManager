# Purpose: Verify Apply command (Ctrl+G) dialog controls, Far substitutions,
# !&q / !&~q quoting for 8.3 short names and leading ~ / #, mark restoration
# with Ctrl+M, and failure mark retention through the live TUI.
Feature: Apply command workflows and shell quoting
  Scenario: Cancel and single-entry substitutions across active and passive panels
    Given a TUI workspace
    And the left file "report.md" contains "draft"
    And the right file "target.bak" contains "backup"
    When I open the TUI at 160 by 30
    And I press "Tab Down Tab Down Ctrl+G"
    Then the screen contains "Apply command"
    And the screen contains "Warning: names are inserted unescaped and run by the shell;"
    When I type "echo skipped"
    And I press "Tab Enter"
    Then the screen does not contain "Apply command"
    When I press "Ctrl+G"
    And I type "echo !.! ! !` !#!.!"
    And I press "Enter"
    Then the screen contains "Apply command: 1 run"
    When I press "Ctrl+O"
    Then the screen contains "echo report.md report md target.bak"
    When I press "Ctrl+O F10 Enter"
    Then the TUI exits successfully

  Scenario: Batch execution keeps 8.3 short names unquoted under !&~q and restores marks with Ctrl+M
    Given a TUI workspace
    And the left file "BACKUP~1.TXT" contains "short"
    And the left file "two words.txt" contains "spaced"
    When I open the TUI at 160 by 30
    And I press "Down Space Space Ctrl+G"
    And I type "echo !.! !&~q"
    And I press "Enter"
    Then the screen contains "Apply command: 2 run"
    When I press "Ctrl+M Ctrl+G"
    And I type "echo step:!.! !&q"
    And I press "Enter"
    Then the screen contains "Apply command: 2 run"
    When I press "Ctrl+O"
    Then the screen contains 'echo BACKUP~1.TXT BACKUP~1.TXT "two words.txt"'
    And the screen contains 'echo step:BACKUP~1.TXT BACKUP~1.TXT "two words.txt"'
    And the screen contains 'echo step:two words.txt "two words.txt"'
    When I press "Ctrl+O F10 Enter"
    Then the TUI exits successfully

  Scenario: Leading hash and tilde names are safely quoted by !&q when run through the shell
    Given a TUI workspace
    And the left file "#note.txt" contains "before-hash"
    And the left file "~" contains "before-tilde"
    When I open the TUI at 160 by 30
    And I press "Down Ctrl+G"
    And I type "echo updated-hash > !&q"
    And I press "Enter"
    Then the screen contains "Apply command: 1 run"
    When I press "F3"
    Then the screen contains "updated-hash"
    And the screen does not contain "before-hash"
    When I press "Esc Down Ctrl+G"
    And I type "echo updated-tilde > !&q"
    And I press "Enter"
    Then the screen contains "Apply command: 1 run"
    When I press "F3"
    Then the screen contains "updated-tilde"
    And the screen does not contain "before-tilde"
    When I press "Esc F10 Enter"
    Then the TUI exits successfully

  Scenario: A failing command keeps its entry marked while an unmarked parent row reports nothing selected
    Given a TUI workspace
    And the left file "keep.txt" contains "data"
    When I open the TUI at 160 by 30
    And I press "Down Space Home Ctrl+G"
    And I type "exit 1"
    And I press "Enter"
    Then the screen contains "Apply command: 1 of 1 failed"
    When I press "Ctrl+G"
    And I type "echo recovered:!.!"
    And I press "Enter"
    Then the screen contains "Apply command: 1 run"
    When I press "Ctrl+G"
    And I type "echo unused"
    And I press "Enter"
    Then the screen contains "Apply command: nothing selected"
    When I press "Ctrl+O"
    Then the screen contains "echo recovered:keep.txt"
    And the screen does not contain "echo unused"
    When I press "Ctrl+O F10 Enter"
    Then the TUI exits successfully
