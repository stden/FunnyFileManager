Feature: Quick view previews the selected entry in the passive panel
  Background:
    Given an isolated acceptance workspace

  Scenario: Ctrl+Q previews text files on the passive panel and follows cursor movement
    Given the left file "alpha.txt" contains "Alpha preview body"
    And the left file "beta.txt" contains "Beta preview body"
    And "alpha.txt" is selected in the left panel
    When I dispatch the keys "Ctrl+Q"
    Then the right panel is a quick view
    And the rendered screen contains "Alpha preview body"
    When I dispatch the keys "Down"
    Then the rendered screen contains "Beta preview body"
    And the rendered screen excludes "Alpha preview body"
    When I dispatch the keys "Ctrl+Q"
    Then the right panel is a file list

  Scenario: Quick view declines binary files inline without opening a modal
    Given a binary left file "blob.bin" containing NUL and "payload"
    And "blob.bin" is selected in the left panel
    When I dispatch the keys "Ctrl+Q"
    Then the right panel is a quick view
    And the overlay is closed
    And the rendered screen contains "Binary file: no text to show"

  Scenario: Quick view shows folder totals when a folder has no description file
    Given a left folder "reports" with 2 files
    And "reports" is selected in the left panel
    When I dispatch the keys "Ctrl+Q"
    Then the right panel is a quick view
    And the rendered screen contains "Total size:"
    And the rendered screen contains "2 files, 0 folders"

  Scenario: Quick view shows README.md when selecting a described folder
    Given a left folder "docs" described by README.md containing "Project Architecture Guide"
    And "docs" is selected in the left panel
    When I dispatch the keys "Ctrl+Q"
    Then the right panel is a quick view
    And the rendered screen contains "Project Architecture Guide"
