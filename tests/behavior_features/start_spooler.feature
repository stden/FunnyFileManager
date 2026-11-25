# Source: README.md:124
Feature: Print Spooler command line
  Background:
    Given an isolated acceptance workspace

  # Completion: docs/knowledge-base/07-core-features/command-line.md:96
  Scenario: Complete the Print Spooler command from its prefix
    When I dispatch the keys ":"
    And I type the text "start-spool"
    And I dispatch the keys "Tab"
    Then the command line equals "start-spooler "

  # Command syntax: src/command/parse.rs:44-45
  Scenario: Reject a service name as an argument
    When I dispatch the keys ":"
    And I type the text "start-spooler OtherService"
    And I submit the invalid built-in command
    Then the rendered screen contains "start-spooler takes no arguments"
    And the overlay is closed
