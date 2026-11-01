# Purpose: Exercises everyday folder-management journeys in isolated temporary workspaces.
Feature: Real-life folder organization
  As a file manager user
  I want common organization workflows to behave predictably
  So that I can structure real work without risking existing files

  Background:
    Given a source directory
    And an empty destination directory

  Scenario: Scaffold a small software project in one workflow
    Given the app is set up with source and destination
    When I scaffold project "weather-cli" with folders "src;tests;docs" through F7
    Then "weather-cli/src" should be a directory in the source
    And "weather-cli/tests" should be a directory in the source
    And "weather-cli/docs" should be a directory in the source

  Scenario: Build a dated photo archive one level at a time
    Given the app is set up with source and destination
    When I create nested folder path "Photos/2026/Tallinn Trip" through F7
    Then "Photos/2026/Tallinn Trip" should be a directory in the source

  Scenario: Prepare an incoming-invoices folder on the other panel
    Given the app is set up with source and destination
    When I create folder "Incoming invoices" on the right panel through F7
    Then "Incoming invoices" should be a directory in the destination
    And "Incoming invoices" should not exist in the source

  Scenario: Add an archive folder without disturbing current work
    Given a file "invoice-1042.pdf" with content "paid" in the source
    And a file "meeting notes.txt" with content "follow up Friday" in the source
    And the app is set up with source and destination
    When I create folder "Processed invoices" through F7
    Then "Processed invoices" should be a directory in the source
    And "invoice-1042.pdf" should still exist in the source
    And "meeting notes.txt" should still exist in the source
