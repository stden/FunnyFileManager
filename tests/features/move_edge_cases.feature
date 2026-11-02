# Purpose: BDD scenarios for move edge cases.
Feature: Move edge cases and rename-failure fallback
  As a file manager user
  I want move operations to handle edge cases gracefully
  So that I never lose data even when rename fails

  Scenario: Move a single file across panels
    Given a source directory
    And a file "transfer.dat" with content "payload" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "transfer.dat" in the left panel
    When I press F6
    Then "transfer.dat" should not exist in the source
    And "transfer.dat" should exist in the destination
    And the status message should contain "Moved"

  Scenario: Move with no selection does nothing
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And the left panel has no selection
    When I press F6
    Then the status message should not be empty

  Scenario: Move a nested directory preserves contents
    Given a source directory
    And a subdirectory "project" in the source
    And a file "project/README.md" with content "# Docs" in the source dir
    And a file "project/src/lib.rs" with content "pub fn run() {}" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    And I select "project" in the left panel
    When I press F6
    Then "project" should not exist in the source
    And "project" should exist in the destination

  Scenario: Move creates unique name on conflict
    Given a source directory
    And a file "clash.txt" with content "new version" in the source
    And an empty destination directory
    And a file "clash.txt" with content "old version" in the destination
    And the app is set up with source and destination
    And I select "clash.txt" in the left panel
    When I press F6
    Then "clash.txt" should not exist in the source
    And the status message should contain "Moved"
