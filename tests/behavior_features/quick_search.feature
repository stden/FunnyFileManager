Feature: Alt-letter quick search jumps the panel cursor to matching entries
  Background:
    Given an isolated acceptance workspace
    And the left panel contains the file "apple.txt"
    And the left panel contains the file "banana.txt"
    And the left panel contains the file "cherry.txt"

  Scenario: Alt-letter moves the cursor to the first entry starting with that letter
    When I dispatch the keys "Alt+b"
    Then the left panel cursor selects "banana.txt"

  Scenario: A different Alt-letter jumps to its own first match
    When I dispatch the keys "Alt+a"
    Then the left panel cursor selects "apple.txt"

  Scenario: Esc exits quick search and keeps the cursor on the matched entry
    When I dispatch the keys "Alt+c"
    Then the left panel cursor selects "cherry.txt"
    And quick search is active
    When I dispatch the keys "Esc"
    Then the left panel cursor selects "cherry.txt"
    And quick search is inactive
