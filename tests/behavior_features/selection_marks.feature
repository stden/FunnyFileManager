Feature: Panel selection marks, totals summary and 8.3 short names

  Scenario: Insert marks entries, advances the cursor and renders the byte summary
    Given an isolated acceptance workspace
    And a sized left file "alpha.rs" of 12 bytes
    And a sized left file "beta.rs" of 17 bytes
    And a sized left file "notes.md" of 100 bytes
    And "alpha.rs" is selected in the left panel
    When I dispatch the keys "Insert Insert"
    Then the left panel cursor selects "notes.md"
    And the left panel has 2 marked entries
    And the rendered screen contains "29 bytes in 2 files"

  Scenario: Mask select, mask unselect, invert and Ctrl+M restore selection
    Given an isolated acceptance workspace
    And a sized left file "app.rs" of 10 bytes
    And a sized left file "app_test.rs" of 20 bytes
    And a sized left file "guide.md" of 30 bytes
    When I dispatch the keys "+ Ctrl+U"
    And I type the text "*.rs"
    And I dispatch the keys "Enter"
    Then the left panel has 2 marked entries
    And the rendered screen contains "30 bytes in 2 files"
    When I dispatch the keys "- Ctrl+U"
    And I type the text "*test*"
    And I dispatch the keys "Enter"
    Then the left panel has 1 marked entries
    And the rendered screen contains "10 bytes in 1 file"
    When I dispatch the keys "*"
    Then the left panel has 2 marked entries
    And the rendered screen contains "50 bytes in 2 files"
    When I clear the left panel marks and press Ctrl+M
    Then the left panel has 2 marked entries
    And the rendered screen contains "50 bytes in 2 files"

  Scenario: Queued Insert burst marks 64 files and updates the selection summary
    Given an isolated acceptance workspace
    And both panels contain 80 numbered documents
    And "file000.txt" is selected in the left panel
    When I queue 64 presses of "Insert"
    And I process the queued input
    Then the left panel has 64 marked entries
    And the left panel cursor selects "file064.txt"
    And the rendered screen contains "in 64 files"

  Scenario: Short 8.3 names renumber colliding stems when sort order reverses
    Given an isolated acceptance workspace
    And the left panel contains the file "longfilename1.txt"
    And the left panel contains the file "longfilename2.txt"
    When I dispatch the keys "Ctrl+N"
    Then the left short name for "longfilename1.txt" is "LONGFI~1.TXT"
    And the left short name for "longfilename2.txt" is "LONGFI~2.TXT"
    When I dispatch the keys "Ctrl+F3"
    Then the left short name for "longfilename2.txt" is "LONGFI~1.TXT"
    And the left short name for "longfilename1.txt" is "LONGFI~2.TXT"
