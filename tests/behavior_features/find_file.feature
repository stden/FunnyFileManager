Feature: Alt+F7 and Alt+F10 filesystem search

  Scenario: Alt+F7 finds Unicode and ASCII text matches, skips binaries, and opens the viewer
    Given an isolated acceptance workspace
    And a nested left file "logs/match.log" containing "2026 INFO Привет МИР ΣΤΡΑΤΟΣ"
    And a nested left file "logs/other.log" containing "2026 INFO routine heartbeat"
    And a binary left file "logs/binary.log" containing NUL and "Привет МИР"
    When I dispatch the keys "Alt+F7 Ctrl+U"
    And I type the text "*.log"
    And I dispatch the keys "Tab"
    And I type the text "мир στρατος"
    And I dispatch the keys "Enter"
    Then the search results dialog lists 1 match
    And the rendered screen contains "match.log"
    And the rendered screen excludes "other.log"
    And the rendered screen excludes "binary.log"
    When I dispatch the keys "Enter"
    Then the viewer overlay is open
    And the rendered screen contains "Привет МИР ΣΤΡΑΤΟΣ"
    When I dispatch the keys "Esc"
    Then the overlay is closed

  Scenario: Alt+F10 finds nested folders by mask and navigates the active panel
    Given an isolated acceptance workspace
    And a nested left file "alpha/target_dir/marker_inside.txt" containing "inside"
    When I dispatch the keys "Alt+F10 Ctrl+U"
    And I type the text "target_*"
    And I dispatch the keys "Enter"
    Then the search results dialog lists 1 match
    When I dispatch the keys "Enter Down"
    Then the overlay is closed
    And the rendered screen contains "target_dir"
    And the left panel cursor selects "marker_inside.txt"
