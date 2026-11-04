Feature: Options menu settings pages change and report panel mode

  Background:
    Given an isolated acceptance workspace

  Scenario: Changing the panel mode to Brief applies immediately and reports the change
    When I open the settings page "File panel modes"
    And I dispatch the keys "Ctrl+A"
    And I type the text "Brief"
    And I dispatch the keys "Enter"
    Then the rendered screen contains "Setting applied"

  Scenario: Changing the panel mode to Detailed applies immediately
    When I open the settings page "File panel modes"
    And I dispatch the keys "Ctrl+A"
    And I type the text "Detailed"
    And I dispatch the keys "Enter"
    Then the rendered screen contains "Setting applied"
