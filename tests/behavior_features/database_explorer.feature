Feature: Database explorer browses SQLite schemas, tables, columns and paged rows
  Background:
    Given an isolated acceptance workspace

  Scenario: Missing databases.yaml opens with a helpful hint and no connections
    Given no databases.yaml exists in the private config directory
    When I open Database explorer from the F9 menu
    Then the database explorer dialog is open
    And the rendered screen contains "No connections: create"
    And the rendered screen contains "databases.yaml"

  Scenario: Malformed databases.yaml with an inline password reports password_env rule
    Given a malformed databases.yaml with an inline password field
    When I open Database explorer from the F9 menu
    Then the database explorer dialog is open
    And the rendered screen contains "databases.yaml"
    And the database notice contains "is not a valid connection"
    And the database notice contains "password_env"

  Scenario: Expanding SQLite shows schemas, views, rows, and column definitions
    Given a configured SQLite database with items table and active_items view
    When I open Database explorer from the F9 menu
    Then the rendered screen contains "shop (sqlite)"
    When I dispatch the keys "Right"
    And the database request completes
    Then the rendered screen contains "main"
    When I dispatch the keys "Down Right"
    Then the rendered screen contains "active_items [view]"
    And the rendered screen contains "items"
    When I dispatch the keys "Down Down Enter"
    And the database request completes
    Then the rendered screen contains "main.items — rows 1–2"
    And the rendered screen contains "widget"
    When I dispatch the keys "Esc Tab"
    And the database request completes
    Then the rendered screen contains "Columns of items"
    And the rendered screen contains "Primary key"
    And the rendered screen contains "INTEGER"

  Scenario: Large tables paginate across 200-row pages with PageDown and PageUp
    Given a configured SQLite database whose items table has 205 rows
    When I open Database explorer from the F9 menu
    And I dispatch the keys "Right"
    And the database request completes
    And I dispatch the keys "Down Right Down Enter"
    And the database request completes
    Then the rendered screen contains "rows 1–200, more follow"
    And the rendered screen contains "row-001"
    When I dispatch the keys "PgDn"
    And the database request completes
    Then the rendered screen contains "rows 201–205"
    And the rendered screen contains "row-205"
    When I dispatch the keys "PgUp"
    And the database request completes
    Then the rendered screen contains "rows 1–200, more follow"

  Scenario: F5 reloads metadata and discovers tables created after opening
    Given a configured SQLite database with items table and active_items view
    When I open Database explorer from the F9 menu
    And I dispatch the keys "Right"
    And the database request completes
    And I dispatch the keys "Down Right"
    And an orders table is added to the SQLite database on disk
    And I dispatch the keys "Up F5"
    And the database request completes
    And I dispatch the keys "Down Right"
    Then the rendered screen contains "orders"

  Scenario: Expanding a missing SQLite database keeps the dialog open with an error
    Given a configured SQLite connection pointing to a missing file
    When I open Database explorer from the F9 menu
    And I dispatch the keys "Right"
    And the database request completes
    Then the database explorer dialog is open
    And the rendered screen contains "broken (sqlite) —"

  Scenario: Esc steps back from views before closing while F10 closes immediately
    Given a configured SQLite database with items table and active_items view
    When I open Database explorer from the F9 menu
    And I dispatch the keys "Right"
    And the database request completes
    And I dispatch the keys "Down Right Down Down Enter"
    And the database request completes
    Then the rendered screen contains "main.items — rows 1–2"
    When I dispatch the keys "Esc"
    Then the database explorer dialog is open
    And the rendered screen contains "items"
    When I dispatch the keys "Esc"
    Then the overlay is closed
    When I open Database explorer from the F9 menu
    And I dispatch the keys "Right"
    And the database request completes
    And I dispatch the keys "Down Right Down Down Enter"
    And the database request completes
    And I dispatch the keys "F10"
    Then the overlay is closed
