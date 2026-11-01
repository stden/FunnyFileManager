Feature: The viewer displays file contents and responds to navigation
  Background:
    Given an isolated acceptance workspace

  Scenario: F3 opens the viewer and shows the file content
    Given a file named "readme.txt" containing "Hello, viewer!"
    When I dispatch the keys "F3"
    Then the overlay is the viewer
    And the rendered screen contains "Hello, viewer!"

  Scenario: Esc closes the viewer
    Given a file named "notes.txt" containing "close me"
    When I dispatch the keys "F3 Esc"
    Then the overlay is closed

  Scenario: F3 a second time also closes the viewer
    Given a file named "data.txt" containing "toggle test"
    When I dispatch the keys "F3 F3"
    Then the overlay is closed

  Scenario: The viewer status line includes the filename
    Given a file named "chapter.txt" containing "Once upon a time"
    When I dispatch the keys "F3"
    Then the rendered screen contains "chapter.txt"

  Scenario: Page Down scrolls past the first page of a long file
    Given a file named "long.txt" with 60 numbered lines
    When I dispatch the keys "F3"
    Then the rendered screen contains "Line 001"
    When I dispatch the keys "PgDn"
    Then the viewer has scrolled past its first line
    And the rendered screen excludes "Line 001"
