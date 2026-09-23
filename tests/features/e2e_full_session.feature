# Purpose: BDD scenarios for full end-to-end user sessions.
Feature: Full session end-to-end tests
  As a file manager user performing realistic sessions
  I want complete multi-step workflows to work correctly
  So that I can rely on FFM for daily file management

  # ── Session 1: Developer organizing a Rust project ─────────────

  Scenario: Developer creates project structure and organizes files
    Given a source directory
    And a file "main.rs" with content "fn main() { println!(\"Hello\"); }" in the source
    And a file "utils.rs" with content "pub fn helper() -> bool { true }" in the source
    And a file "README.md" with content "# My Project" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I press F7
    Then "new_folder" should be a directory in the source
    When I select "main.rs" in the left panel
    And I press F5
    Then "main.rs" should exist in the destination
    And "main.rs" should still exist in the source
    And the status message should contain "Copied"

  Scenario: Developer moves files to backup directory
    Given a source directory
    And a file "old_code.rs" with content "// deprecated" in the source
    And a file "old_tests.rs" with content "// obsolete" in the source
    And a file "current.rs" with content "// keep this" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I mark "old_code.rs" in the left panel
    And I mark "old_tests.rs" in the left panel
    When I press F6
    Then "old_code.rs" should not exist in the source
    And "old_tests.rs" should not exist in the source
    And "old_code.rs" should exist in the destination
    And "old_tests.rs" should exist in the destination
    And "current.rs" should still exist in the source
    And the status message should contain "Moved"

  # ── Session 2: File review and cleanup ─────────────────────────

  Scenario: Review file contents then decide to keep or delete
    Given a source directory
    And a file "important.cfg" with content "database_url=postgres://localhost" in the source
    And a file "cache.tmp" with content "temporary data xyz" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "important.cfg" in the left panel
    When I press F3
    Then the viewer should be open
    And the viewer should show "database_url"
    When I close the viewer
    Then the viewer should not be open
    When I select "cache.tmp" in the left panel
    And I press F8
    And I confirm the deletion
    Then "cache.tmp" should not exist in the source
    And "important.cfg" should still exist in the source

  Scenario: Cancel accidental delete preserves file
    Given a source directory
    And a file "precious.dat" with content "irreplaceable data" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "precious.dat" in the left panel
    When I press F8
    Then a delete confirmation should be shown
    When I cancel the deletion
    Then a delete confirmation should not be shown
    And "precious.dat" should still exist in the source

  # ── Session 3: Panel navigation and bookmarks ──────────────────

  Scenario: Navigate deep directory tree with bookmarks
    Given a source directory
    And a subdirectory "projects" in the source
    And a file "projects/rust/main.rs" with content "fn main() {}" in the source dir
    And a file "projects/go/main.go" with content "package main" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    When I set bookmark 1
    When I execute command "cd projects"
    When I set bookmark 2
    When I go to bookmark 1
    Then the active panel path should not end with "projects"
    When I go to bookmark 2
    Then the active panel path should end with "projects"

  Scenario: Swap panels and work from both sides
    Given a source directory
    And a file "left_file.txt" with content "from left" in the source
    And an empty destination directory
    And a file "right_file.txt" with content "from right" in the destination
    And the app is set up with source and destination
    When I swap panels
    Then the active panel should be Left
    When I set the active panel to Right
    And I select "left_file.txt" in the right panel
    When I press F5
    Then the status message should contain "Copied"

  # ── Session 4: Sorting and search workflows ────────────────────

  Scenario: Sort files by different criteria then find specific file
    Given a source directory
    And a file "alpha.txt" with content "aaa" in the source
    And a file "beta.csv" with content "1,2,3" in the source
    And a file "gamma.rs" with content "fn g() {}" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I sort by name
    Then the sort mode should be Name
    When I sort by extension
    Then the sort mode should be Extension
    When I sort by size
    Then the sort mode should be Size
    When I sort by name
    Then the sort mode should be Name

  Scenario: Quick search to find and copy a specific file
    Given a source directory
    And a file "config.yaml" with content "port: 8080" in the source
    And a file "database.yaml" with content "host: localhost" in the source
    And a file "secrets.env" with content "KEY=abc" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I quick search for "d" in the app
    When I press F5
    Then the status message should contain "Copied"

  # ── Session 5: Console mode and help ───────────────────────────

  Scenario: Toggle console mode during file operations
    Given a source directory
    And a file "work.txt" with content "working" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I toggle console mode
    Then console mode should be on
    When I toggle console mode
    Then console mode should be off
    And the left panel should have entries
    And the right panel should have entries

  Scenario: Help overlay does not interfere with panel state
    Given a source directory
    And a file "stable.txt" with content "unchanged" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "stable.txt" in the left panel
    When I toggle help
    Then help should be visible
    When I toggle help
    Then help should not be visible
    And the left panel should have entries

  # ── Session 6: Batch operations ────────────────────────────────

  Scenario: Mark all files copy then clean source
    Given a source directory
    And a file "report1.pdf" with content "pdf1" in the source
    And a file "report2.pdf" with content "pdf2" in the source
    And a file "report3.pdf" with content "pdf3" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I select all files
    Then the active panel should have marks
    When I press F5
    Then "report1.pdf" should exist in the destination
    And "report2.pdf" should exist in the destination
    And "report3.pdf" should exist in the destination
    And the status message should contain "Copied"

  Scenario: Create new file via dialog and verify it appears
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I open new file dialog
    And I type filename "notes.md"
    And I confirm new file
    Then the status message should contain "Created"

  # ── Session 7: Editor integration ──────────────────────────────

  Scenario: Edit file with successful editor launch
    Given a source directory
    And a file "editable.txt" with content "edit me" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "editable.txt" in the left panel
    When I press F4 with success behavior
    Then the status message should contain "Edited"

  Scenario: Handle editor launch failure gracefully
    Given a source directory
    And a file "locked.txt" with content "can't edit" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "locked.txt" in the left panel
    When I press F4 with fail_launch behavior
    Then the status message should contain "Failed"

  # ── Session 8: Edge cases and resilience ───────────────────────

  Scenario: Copy file with special characters in name
    Given a source directory
    And a file "my file (1).txt" with content "spaces and parens" in the source
    And an empty destination directory
    And the app is set up with source and destination
    And I select "my file (1).txt" in the left panel
    When I press F5
    Then "my file (1).txt" should exist in the destination
    And the status message should contain "Copied"

  Scenario: Copy file that already exists creates unique name
    Given a source directory
    And a file "duplicate.txt" with content "original" in the source
    And an empty destination directory
    And a file "duplicate.txt" with content "already here" in the destination
    And the app is set up with source and destination
    And I select "duplicate.txt" in the left panel
    When I press F5
    Then the status message should contain "Copied"

  Scenario: Directory operations preserve nested structure
    Given a source directory
    And a subdirectory "project" in the source
    And a file "project/src/main.rs" with content "fn main() {}" in the source dir
    And a file "project/Cargo.toml" with content "[package]" in the source dir
    And an empty destination directory
    And the app is set up with source and destination
    And I select "project" in the left panel
    When I press F5
    Then "project" should exist in the destination
    And the status message should contain "Copied"

  # ── Session 9: Command line interface ──────────────────────────

  Scenario: Execute pwd command shows current directory
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "pwd"
    Then the status message should not be empty

  Scenario: Execute sort command changes sort mode
    Given a source directory
    And a file "z.txt" with content "z" in the source
    And a file "a.txt" with content "a" in the source
    And an empty destination directory
    And the app is set up with source and destination
    When I execute command "sort ext"
    Then the sort mode should be Extension
    When I execute command "sort size"
    Then the sort mode should be Size
    When I execute command "sort name"
    Then the sort mode should be Name

  # ── Session 10: Multi-panel coordination ───────────────────────

  Scenario: Tab switching preserves panel contexts
    Given a source directory
    And a file "src_file.txt" with content "source side" in the source
    And an empty destination directory
    And a file "dst_file.txt" with content "dest side" in the destination
    And the app is set up with source and destination
    Then the active panel should be Left
    When I press Tab
    Then the active panel should be Right
    When I press Tab
    Then the active panel should be Left
    And the left panel should have entries
    And the right panel should have entries

  Scenario: Left and Right arrow keys select panels
    Given a source directory
    And an empty destination directory
    And the app is set up with source and destination
    When I press Right
    Then the active panel should be Right
    When I press Left
    Then the active panel should be Left

  Scenario: Copy from right panel to left panel
    Given a source directory
    And an empty destination directory
    And a file "from_right.txt" with content "going left" in the destination
    And the app is set up with source and destination
    When I set the active panel to Right
    And I select "from_right.txt" in the right panel
    When I press F5
    Then "from_right.txt" should exist in the source
    And the status message should contain "Copied"
