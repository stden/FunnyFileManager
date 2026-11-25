Feature: Apply command (Ctrl+G) substitutes selections into shell commands

  Background:
    Given an isolated acceptance workspace

  Scenario: Apply command on a panel with no file selected reports nothing selected
    When I dispatch the keys "Ctrl+G"
    And I type the text "echo !.!"
    And I dispatch the keys "Enter"
    Then the rendered screen contains "nothing selected"
    And the overlay is closed
