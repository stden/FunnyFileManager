# Purpose: BDD scenarios for console mode behavior.
Feature: Console mode advanced scenarios
  As a file manager user
  I want console mode to toggle cleanly
  So that I can switch between file management and console view

  Scenario: Toggle console mode on
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    Then console mode should be off
    When I toggle console mode
    Then console mode should be on

  Scenario: Toggle console mode off
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I toggle console mode
    Then console mode should be on
    When I toggle console mode
    Then console mode should be off

  Scenario: Double toggle returns to original state
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    Then console mode should be off
    When I toggle console mode
    When I toggle console mode
    Then console mode should be off

  Scenario: Console mode does not affect panel data
    Given a source directory
    And a file "persist.txt" with content "still here" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I toggle console mode
    Then console mode should be on
    When I toggle console mode
    Then console mode should be off
    And the left panel should have entries

  Scenario: Panel operations work after console mode
    Given a source directory
    And a file "after_console.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I toggle console mode
    When I toggle console mode
    And I select "after_console.txt" in the left panel
    When I press F5
    Then "after_console.txt" should exist in the destination
