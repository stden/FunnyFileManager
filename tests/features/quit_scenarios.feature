# Purpose: BDD scenarios for quit and exit flows.
Feature: Application quit scenarios
  As a file manager user
  I want multiple ways to quit the application
  So that I can exit quickly from any state

  Scenario: quit command exits
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "quit"
    Then the app should be quitting

  Scenario: exit command exits
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "exit"
    Then the app should be quitting

  Scenario: Setting should_quit flag works
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I set should_quit
    Then the app should be quitting

  Scenario: Close help then quit
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I toggle help
    Then help should be visible
    When I toggle help
    Then help should not be visible
    When I execute command "quit"
    Then the app should be quitting
