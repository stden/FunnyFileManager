# Purpose: BDD scenarios for dual-panel behavior.
Feature: Panel switching and swapping
  As a file manager user
  I want to switch and swap panels reliably
  So that I can work with both panels efficiently

  Scenario: Tab switches from left to right
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    Then the active panel should be Left
    When I press Tab
    Then the active panel should be Right

  Scenario: Tab switches from right to left
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Tab
    Then the active panel should be Right
    When I press Tab
    Then the active panel should be Left

  Scenario: Double Tab returns to original panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    Then the active panel should be Left
    When I press Tab
    When I press Tab
    Then the active panel should be Left

  Scenario: Left arrow selects left panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Tab
    Then the active panel should be Right
    When I press Left
    Then the active panel should be Left

  Scenario: Right arrow selects right panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Right
    Then the active panel should be Right

  Scenario: Swap panels exchanges contents
    Given a source directory
    And a file "left_file.txt" with content "L" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I swap panels
    Then the active panel should be Left
    And the left panel should have entries

  Scenario: Double swap restores original state
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I swap panels
    When I swap panels
    Then the active panel should be Left
    And the left panel should have entries

  Scenario: Tab after swap works correctly
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I swap panels
    When I press Tab
    Then the active panel should be Right
    When I press Tab
    Then the active panel should be Left
