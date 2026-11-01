Feature: F4 editor save, case-insensitive search and repeat search

  Scenario: F2 saves buffer edits to disk and allows Esc to close without prompting
    Given an isolated acceptance workspace
    And a file named "config.toml" containing "port = 8080"
    When I dispatch the keys "F4 End"
    And I type the text "0"
    And I dispatch the keys "F2 Esc"
    Then the overlay is closed
    And the left file "config.toml" equals "port = 80800"

  Scenario: F7 case-insensitive search and Shift+F7 repeat search walk Unicode and ASCII matches
    Given an isolated acceptance workspace
    And a multi-line file "notes.txt" with lines:
      | header — αβγ                 |
      | first Target_Token here      |
      | middle Привет target_token!  |
      | footer line                  |
    When I dispatch the keys "F4 F7 Ctrl+U"
    And I type the text "TARGET_TOKEN"
    And I dispatch the keys "Enter"
    Then the editor caret sits at line 1 and column 6
    When I dispatch the keys "Shift+F7"
    Then the editor caret sits at line 2 and column 14
    When I dispatch the keys "Shift+F7"
    Then the editor caret sits at line 1 and column 6

  Scenario: Dismissing the search not-found warning does not insert the key into the buffer
    Given an isolated acceptance workspace
    And a file named "plain.txt" containing "unchanged"
    When I dispatch the keys "F4 F7 Ctrl+U"
    And I type the text "missing_needle"
    And I dispatch the keys "Enter"
    Then the editor shows the not-found warning
    When I dispatch the keys "x"
    Then the editor buffer equals "unchanged"
