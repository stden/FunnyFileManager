Feature: Panel sorting orders entries predictably

  Background:
    Given an isolated acceptance workspace

  Scenario: Sort by name places entries in alphabetical order
    Given the left panel contains the file "zebra.txt"
    And the left panel contains the file "apple.txt"
    And the left panel contains the file "mango.txt"
    And the left panel starts with reverse name sorting
    When I dispatch the keys "Ctrl+F3"
    Then the left panel entries are sorted alphabetically
    When I dispatch the keys "Ctrl+F3"
    Then the left panel entries are sorted in reverse alphabetical order

  Scenario: Sort by extension groups files by type
    Given the left panel contains the file "alpha.txt"
    And the left panel contains the file "beta.rs"
    And the left panel contains the file "gamma.txt"
    And the left panel contains the file "delta.rs"
    When I dispatch the keys "Ctrl+F4"
    Then files with the same extension are grouped together
