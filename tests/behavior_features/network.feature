Feature: Network monitoring presents rates, interfaces and diagnostic health
  Background:
    Given an isolated acceptance workspace
    And the task list has two network counter samples

  Scenario: Rates use the elapsed counter interval and the summary shows traffic
    Then network traffic rates come from the elapsed sample interval
    And the rendered screen contains "Net["
    And the rendered screen contains "2.0M"
    And the rendered screen contains "250K"

  Scenario: Tab switches to network and F2 includes inactive adapters
    When I dispatch the keys "Tab"
    Then the rendered screen contains "Task list · Network"
    And the rendered screen contains "Ethernet"
    And the rendered screen excludes "Wi-Fi"
    When I dispatch the keys "F2"
    Then the rendered screen contains "Wi-Fi"
    And the rendered screen contains "lo"
    And the rendered screen contains "0/2"

  Scenario: The interface dialog shows address, link speed and counters
    When I dispatch the keys "F12 F3"
    Then the rendered screen contains "Network interface"
    And the rendered screen contains "192.0.2.10"
    And the rendered screen contains "1000 Mbps"
    When I dispatch the keys "Esc F12"
    Then the rendered screen excludes "Network interface"
    And the rendered screen contains "worker"

  Scenario Outline: Diagnostic results produce visible health labels
    When the diagnostic sweep reports <outcome>
    And I dispatch the keys "Tab"
    Then the rendered screen contains "Latency & DNS Diagnostics"
    And the rendered screen contains "<health>"
    Examples:
      | outcome  | health  |
      | healthy  | OK      |
      | timeout  | TIMEOUT |
      | slow-DNS | SLOW    |

  Scenario: Process control shortcuts do nothing on the network tab
    When I dispatch the keys "Tab ] F8 F7"
    Then no process control was requested
    And the rendered screen contains "Task list · Network"

  Scenario: Holding refresh pauses diagnostics but manual refresh is still available
    When the diagnostic sweep reports healthy
    And I dispatch the keys "Tab F11"
    Then network diagnostics are held until an explicit refresh
    When I dispatch the keys "Ctrl+R"
    Then an explicit diagnostic refresh is due
