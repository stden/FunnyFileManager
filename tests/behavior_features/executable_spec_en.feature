Feature: High-level executable specification of FunnyFileManager core requirements

  Background:
    Given an isolated FFM specification workspace
    And in the left panel there is a file "alpha.txt" with content "alpha body"
    And in the left panel there is a file "beta.rs" with content "fn main() {}"
    And in the left panel there is a file "gamma.md" with content "# Title"

  Scenario: Dual-panel focus switching, swapping and folder lifecycle
    Then focus is on the left panel
    And the active domain context is "Panels"
    When the user switches focus to the opposite panel
    Then focus is on the right panel
    When the user switches focus to the opposite panel
    And the user creates folder "workspace-sub" in the active panel
    Then in the left panel folder "workspace-sub" exists
    And the cursor in the left panel rests on "workspace-sub"
    When the user enters folder "workspace-sub" in the active panel
    And the user navigates to the parent folder
    Then the cursor in the left panel rests on "workspace-sub"
    When the user swaps the panels
    Then in the right panel folder "workspace-sub" exists

  Scenario: Safe file copy, move, rename, conflict protection and deletion
    Given the cursor in the left panel rests on "alpha.txt"
    When the user copies the current entry to the opposite panel
    Then in the left panel file "alpha.txt" has content "alpha body"
    And in the right panel file "alpha.txt" has content "alpha body"
    Given the cursor in the left panel rests on "beta.rs"
    When the user renames the current entry to "alpha.txt"
    Then the operation is rejected because the target already exists
    And in the left panel file "alpha.txt" has content "alpha body"
    And in the left panel file "beta.rs" has content "fn main() {}"
    When the user renames the current entry to "renamed.rs"
    Then in the left panel entry "beta.rs" is absent
    And in the left panel file "renamed.rs" has content "fn main() {}"
    When the user moves the current entry to the opposite panel
    Then in the left panel entry "renamed.rs" is absent
    And in the right panel file "renamed.rs" has content "fn main() {}"
    Given the cursor in the left panel rests on "gamma.md"
    When the user starts deleting the current entry and cancels
    Then in the left panel file "gamma.md" has content "# Title"
    When the user deletes the current entry with confirmation
    Then in the left panel entry "gamma.md" is absent

  Scenario: Marking, selection inversion and domain sorting
    Given the cursor in the left panel rests on "alpha.txt"
    When the user marks the current entry
    Then the left panel has marked entries "alpha.txt"
    When the user inverts the selection in the active panel
    Then the left panel has marked entries "beta.rs, gamma.md"
    When the user sorts the active panel by extension
    Then the left panel entries are ordered as "gamma.md, beta.rs, alpha.txt"
    When the user sorts the active panel by name
    Then the left panel entries are ordered as "alpha.txt, beta.rs, gamma.md"

  Scenario: Built-in viewer and editor lifecycle
    Given the cursor in the left panel rests on "gamma.md"
    When the user opens the current entry for viewing
    Then the active domain context is "Viewer"
    When the user closes the active modal tool
    Then the active domain context is "Panels"
    When the user opens the current entry for editing
    Then the active domain context is "Editor"
    When the user appends line "Appended from spec" in the editor and saves
    And the user closes the active modal tool
    Then the active domain context is "Panels"
    And in the left panel file "gamma.md" has content "# Title\nAppended from spec"
