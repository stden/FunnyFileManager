Feature: Things catalog lists the machines of a read and imports the chosen ones as sessions
  Background:
    Given an isolated acceptance workspace
    And a private ffm.yml for the Things sessions

  Scenario: Without THINGS_URL and THINGS_API_TOKEN the dialog names both and starts no read
    Given the Things settings are unset
    When I choose the Things catalog from the F9 menu
    Then the Things catalog dialog is open
    And the rendered screen contains "Set THINGS_URL and THINGS_API_TOKEN to browse Things"
    And the rendered screen excludes "Loading Things..."
    And no Things read starts

  Scenario: The catalog lists the machines that the read returns
    Given the Things settings are configured
    When I choose the Things catalog from the F9 menu
    Then the rendered screen contains "Loading Things..."
    When the Things read returns the machines "alpha beta gamma"
    Then the Things list shows a row for "alpha"
    And the Things list shows a row for "gamma"
    And the rendered screen contains "Things catalog"

  Scenario: Typing narrows the list to the machines the filter matches
    Given the Things settings are configured
    And the Things catalog lists the machines "alpha beta gamma"
    When I dispatch the keys "b e t a"
    Then the rendered screen contains "filter: beta"
    And the Things list shows a row for "beta"
    And the Things list has no row for "alpha"

  Scenario: Space marks the machine under the cursor and Enter imports that machine
    Given the Things settings are configured
    And the Things catalog lists the machines "alpha beta gamma"
    When I dispatch the keys "Down Space"
    Then the Things list marks the row for "beta"
    When I dispatch the keys "Enter"
    Then the rendered screen contains "Things: 1 imported, 0 skipped"
    And the overlay is closed
    And the saved Things sessions are "beta"

  Scenario: A mark survives a filter that hides its machine, and Enter imports only the marked machine
    Given the Things settings are configured
    And the Things catalog lists the machines "alpha beta gamma"
    When I dispatch the keys "Down Space g a m m a"
    Then the rendered screen contains "filter: gamma"
    And the Things list has no row for "beta"
    When I dispatch the keys "Enter"
    Then the rendered screen contains "Things: 1 imported, 0 skipped"
    And the saved Things sessions are "beta"

  Scenario: With nothing marked, Enter imports every machine the filter shows
    Given the Things settings are configured
    And the Things catalog lists the machines "alpha beta gamma"
    When I dispatch the keys "b e t a Enter"
    Then the rendered screen contains "Things: 1 imported, 0 skipped"
    And the saved Things sessions are "beta"

  Scenario: Enter with no machine shown closes the dialog and saves nothing
    Given the Things settings are configured
    And the Things catalog lists the machines "alpha beta gamma"
    When I dispatch the keys "z z z Enter"
    Then the rendered screen contains "Things: no machines to import"
    And the overlay is closed
    And no Things sessions are saved

  Scenario: Esc closes the Things dialog and saves nothing
    Given the Things settings are configured
    And the Things catalog lists the machines "alpha beta gamma"
    When I dispatch the keys "Down Space Esc"
    Then the overlay is closed
    And no Things sessions are saved
