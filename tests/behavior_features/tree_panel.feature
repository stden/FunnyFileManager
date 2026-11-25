Feature: The folder tree panel navigates folders and synchronizes panels
  Background:
    Given an isolated acceptance workspace

  Scenario: Ctrl+T toggles the folder tree revealed at the active directory
    Given a left folder hierarchy with "alpha/nested" and "zebra"
    When I dispatch the keys "Ctrl+T"
    Then the left panel is a folder tree
    And the tree cursor selects folder "left"
    When I dispatch the keys "Ctrl+T"
    Then the left panel is a file list

  Scenario: Expanding and moving in the tree updates the passive panel and Enter opens the folder
    Given a left folder hierarchy with "alpha/nested" and "zebra"
    When I dispatch the keys "Ctrl+T Right Right"
    Then the tree cursor selects folder "alpha"
    And the right panel is at left subfolder "alpha"
    When I dispatch the keys "Down"
    Then the tree cursor selects folder "zebra"
    And the right panel is at left subfolder "zebra"
    When I dispatch the keys "Left"
    Then the tree cursor selects folder "left"
    When I dispatch the keys "Right Enter"
    Then the left panel is a file list
    And the left panel is at left subfolder "alpha"

  Scenario: Typing letters jumps to a matching folder and file-panel keys explain how to act
    Given a left folder hierarchy with "alpha/nested" and "zebra"
    When I dispatch the keys "Ctrl+T Right z"
    Then the tree cursor selects folder "zebra"
    When I dispatch the keys "Esc F7"
    Then the rendered screen contains "Tree panel: press Enter to open the folder"
    When I dispatch the keys "F8"
    Then the rendered screen contains "Tree panel: press Enter to open the folder"
    And the overlay is closed
