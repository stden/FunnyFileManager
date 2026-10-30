Feature: Responsive panels preserve every queued action
  Background:
    Given an isolated acceptance workspace
    And both panels contain 160 numbered documents

  Scenario: A long keyboard burst yields frames and preserves the panel switch
    When I queue 128 presses of "Down"
    And I queue the keys "Tab Down"
    And I process the queued input
    Then the left panel cursor selects "file127.txt"
    And the right panel cursor selects "file000.txt"
    And queued navigation yields frames within its processing budget
    And the terminal presents complete synchronized frames

  Scenario: Wheel bursts target the panel under the pointer
    When I queue 64 downward wheel ticks over the right panel
    And I process the queued input
    Then the left panel cursor selects ".."
    And the right panel cursor selects "file063.txt"
    And queued navigation yields frames within its processing budget

  Scenario: A queued destructive command acts on the final selected document
    When I queue 80 presses of "Down"
    And I queue the keys "F8"
    And I process the queued input
    Then the left panel cursor selects "file079.txt"
    And the rendered screen contains "file079.txt"
    And the rendered screen contains "Delete"
    When I dispatch the keys "Esc F3"
    Then the rendered screen contains "document 79"

  Scenario: Batching respects configured actions rather than physical arrows
    Given j moves down and Down opens deletion confirmation
    When I queue 64 presses of "j"
    And I queue the keys "Down"
    And I process the queued input
    Then the left panel cursor selects "file063.txt"
    And the rendered screen contains "Delete"
    And queued navigation yields frames within its processing budget

  Scenario: Resize and viewer commands keep their place in the queue
    When I queue 64 presses of "Down"
    And I queue a terminal resize to 80 by 20
    And I queue the keys "Down Down F3"
    And I process the queued input
    Then the left panel cursor selects "file065.txt"
    And the rendered terminal size is 80 by 20
    And the rendered screen contains "document 65"

  Scenario: A failed draw still releases synchronized presentation
    When the backend refuses the next repaint
    Then the repaint failure is reported
    And the terminal presents complete synchronized frames
