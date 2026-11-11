# Purpose: BDD scenarios for drive selection behavior.
Feature: Drive selection overlay
  As a file manager user
  I want to select drives from an overlay
  So that I can navigate to different root directories

  Scenario: Open drive selection for left panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open drive selection for Left
    Then drive selection should be visible

  Scenario: Open drive selection for right panel
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open drive selection for Right
    Then drive selection should be visible
