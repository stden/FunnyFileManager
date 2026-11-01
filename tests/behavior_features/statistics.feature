Feature: Action statistics measure UI waiting without recording private input
  Background:
    Given an isolated acceptance workspace

  Scenario: External program waiting is excluded while rendering remains measured
    When an external fixture command runs during a measured action
    Then the external wait is excluded from recorded input latency

  Scenario: Pasted commands are represented by their action context only
    When I paste a private fixture command into the command line
    Then saved statistics identify the action without its contents or paths
