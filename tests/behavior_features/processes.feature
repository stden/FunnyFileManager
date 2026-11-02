Feature: Process monitoring and Windows priority controls use verified targets
  Background:
    Given an isolated acceptance workspace

  Scenario Outline: A class step is reflected immediately and sent once
    Given the task list contains a Windows process with Normal priority
    When I dispatch the keys "<key>"
    Then the fixture process priority is <class>
    And only class <class> was requested from the process host
    Examples:
      | key | class       |
      | ]   | AboveNormal |
      | [   | BelowNormal |

  Scenario Outline: Boundary and Realtime classes are refused before any control call
    Given the task list contains a Windows process with <class> priority
    When I dispatch the keys "<key>"
    Then no process control was requested
    And the fixture process priority is <class>
    And the rendered screen contains "<reason>"
    Examples:
      | class    | key | reason   |
      | High     | ]   | limit    |
      | Idle     | [   | limit    |
      | Realtime | ]   | Realtime |

  Scenario: Refused priority changes retain the original class and show an error
    Given the task list contains a Windows process with Normal priority
    And the process host refuses priority changes
    When I dispatch the keys "]"
    Then only class AboveNormal was requested from the process host
    And the fixture process priority is Normal
    And the rendered screen contains "Error"

  Scenario: Cancelling process termination has no effect
    Given the task list contains a Windows process with Normal priority
    When I dispatch the keys "F8"
    Then termination is awaiting confirmation
    And no process control was requested
    When I dispatch the keys "Esc"
    Then no process control was requested
    And the rendered screen contains "worker"

  Scenario: Confirmed termination reaches only the selected fixture process
    Given the task list contains a Windows process with Normal priority
    When I dispatch the keys "F8 Enter"
    Then only the selected fixture process received termination
