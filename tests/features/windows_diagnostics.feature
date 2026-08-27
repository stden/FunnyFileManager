# Purpose: BDD scenarios for Windows diagnostics and platform behavior.
Feature: Windows diagnostics commands
  As a file manager user
  I want to run Windows diagnostic commands
  So that I can check system health from within the file manager

  Scenario: winerrors command on non-Windows shows unavailable message
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "winerrors"
    Then the status message should contain "only available on Windows"

  Scenario: wincheck command on non-Windows shows unavailable message
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "wincheck"
    Then the status message should contain "only available on Windows"
