# Purpose: BDD scenarios for Ctrl+Z, the Far-style Describe files dialog.
Feature: Describe files
  As a file manager user
  I want Ctrl+Z to ask for a description of the file under the cursor
  So that the Descriptions columns show what a file is for

  Background:
    Given a source directory
    And an empty destination directory

  Scenario: Ctrl+Z writes the description of the file under the cursor
    Given a file "notes.txt" with content "" in the source
    And the app is set up with source and destination
    And I select "notes.txt" in the left panel
    When I describe the selection as "Quarterly notes" through Ctrl+Z
    Then "Descript.ion" in the source should have the line "notes.txt Quarterly notes"

  Scenario: A name with a space is quoted as Far writes it
    Given a file "b c.txt" with content "" in the source
    And the app is set up with source and destination
    And I select "b c.txt" in the left panel
    When I describe the selection as "Spaced" through Ctrl+Z
    Then the description of "b c.txt" in the source should be "Spaced"
    And "Descript.ion" in the source should quote "b c.txt" before "Spaced"

  Scenario: Marked files are described one after another
    Given a file "a.txt" with content "" in the source
    And a file "b.txt" with content "" in the source
    And the app is set up with source and destination
    And I mark "a.txt" in the left panel
    And I mark "b.txt" in the left panel
    When I describe the marked files as "One" and "Two" through Ctrl+Z
    Then the description of "a.txt" in the source should be "One"
    And the description of "b.txt" in the source should be "Two"

  Scenario: Escape leaves the description file alone
    Given a file "notes.txt" with content "" in the source
    And the app is set up with source and destination
    And I select "notes.txt" in the left panel
    When I cancel the Describe files dialog after typing "Lost"
    Then "Descript.ion" should not exist in the source
