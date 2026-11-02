Feature: Disk cleanup measures developer caches and removes only confirmed targets
  Background:
    Given an isolated acceptance workspace

  Scenario: Default disk cleanup registers Cargo registry and platform caches as opt-in path entries
    Then the default disk cleanup checklist includes Cargo registry and platform caches as opt-in path entries

  Scenario: Interactive disk cleanup removes ticked caches while preserving unticked ones
    Given a sandboxed disk cleanup checklist with populated developer caches
    When I open the sandboxed disk cleanup dialog
    Then "Temp files" and "JVM heap dumps" are checked while "Cargo registry cache" and "Gradle caches" are unchecked
    When I dispatch the keys "Down Space Tab Tab Tab Enter"
    Then the overlay is closed
    And the rendered screen contains "Freed"
    And the sandboxed cleanup removes "Cargo registry cache" and "Temp files" contents and heap dumps while keeping "Gradle caches"

  Scenario: Esc cancels disk cleanup without removing checked caches
    Given a sandboxed disk cleanup checklist with populated developer caches
    When I open the sandboxed disk cleanup dialog
    And I dispatch the keys "Esc"
    Then the overlay is closed
    And the rendered screen contains "Cleanup cancelled"
    And all sandboxed developer cache files remain intact
