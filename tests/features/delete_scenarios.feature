# Purpose: BDD scenarios for delete confirmations and deletion paths.
Feature: Delete operation scenarios
  As a file manager user
  I want delete operations to be safe and require confirmation
  So that I never accidentally lose important files

  Scenario: Delete requires confirmation dialog
    Given a source directory
    And a file "important.txt" with content "data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "important.txt" in the left panel
    When I press F8
    Then a delete confirmation should be shown

  Scenario: Confirm delete removes file
    Given a source directory
    And a file "doomed.txt" with content "bye" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "doomed.txt" in the left panel
    When I press F8
    And I confirm the deletion
    Then "doomed.txt" should not exist in the source

  Scenario: Cancel delete preserves file
    Given a source directory
    And a file "safe.txt" with content "keep" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "safe.txt" in the left panel
    When I press F8
    And I cancel the deletion
    Then "safe.txt" should still exist in the source

  Scenario: Delete directory recursively
    Given a source directory
    And a subdirectory "rmdir" in the source with file "inner.txt"
    And an empty destination directory
    And the app is set up with source and destination
    And I select "rmdir" in the left panel
    When I press F8
    And I confirm the deletion
    Then "rmdir" should not exist in the source

  Scenario: Delete deeply nested directory
    Given a source directory
    And a subdirectory "deep" in the source
    And a file "deep/level1.txt" with content "L1" in the source dir
    And a file "deep/sub/level2.txt" with content "L2" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    And I select "deep" in the left panel
    When I press F8
    And I confirm the deletion
    Then "deep" should not exist in the source

  Scenario: Delete with no selection does nothing
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    And the left panel has no selection
    When I press F8
    Then a delete confirmation should not be shown

  Scenario: Delete marked files removes all
    Given a source directory
    And a file "del1.txt" with content "1" in the source
    And a file "del2.txt" with content "2" in the source
    And a file "keep.txt" with content "k" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I mark "del1.txt" in the left panel
    And I mark "del2.txt" in the left panel
    When I press F8
    And I confirm the deletion
    Then "del1.txt" should not exist in the source
    And "del2.txt" should not exist in the source
    And "keep.txt" should still exist in the source
