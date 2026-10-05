# Purpose: BDD scenarios for Panel On/Off (Ctrl+F1 / Ctrl+F2).
Feature: Panel On/Off
  As a file manager user
  I want to take one panel off the screen
  So that the other panel gets the whole width for a long listing

  Scenario: Ctrl+F2 hides the right panel and the left one takes the width
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Ctrl+F2
    Then only the Left panel should be on screen
    And the source directory should be drawn
    And the destination directory should not be drawn
    And the status message should contain "Right panel off"

  Scenario: Hiding the active panel hands the cursor to the other
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    Then the active panel should be Left
    When I press Ctrl+F1
    Then only the Right panel should be on screen
    And the active panel should be Right
    And the destination directory should be drawn
    And the source directory should not be drawn

  Scenario: Ctrl+F1 twice brings the left panel back
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Ctrl+F1
    And I press Ctrl+F1
    Then both panels should be on screen
    And the status message should contain "Left panel on"

  Scenario: Tab and Right never land on a hidden panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Ctrl+F2
    And I press Tab
    Then the active panel should be Left
    When I press Right
    Then the active panel should be Left
    When I press Ctrl+F2
    And I press Tab
    Then the active panel should be Right

  Scenario: The last panel on screen cannot be hidden
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Ctrl+F2
    And I press Ctrl+F1
    Then only the Left panel should be on screen
    And the status message should contain "the last panel cannot be hidden"

  Scenario: Ctrl+U does not swap while a panel is hidden
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Ctrl+F2
    And I swap panels
    Then the active panel should be at the source directory
    And the destination directory should not be drawn
    And the status message should contain "not swapped while one is hidden"
