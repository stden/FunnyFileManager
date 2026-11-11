Feature: Network diagnostics finish off the UI thread
  Scenario: Fixture sampler and prober publish a healthy diagnostic result
    Given an isolated acceptance workspace
    And a fixture network worker is installed
    When background task sampling and diagnostic probes complete
    And I dispatch the keys "Tab"
    Then the rendered screen contains "Ethernet"
    And the rendered screen contains "OK"
    And background probe latency is recorded without probe payloads
    And no process control was requested
