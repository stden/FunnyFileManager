# Purpose: BDD scenarios for glob patterns and find-and-replace reporting.
Feature: Pattern matching via MCP tools
  As an AI assistant
  I want patterns to mean what they say
  So that I never act on the wrong files

  Background:
    Given a temporary directory

  Scenario: Glob honours the directory part of the pattern
    Given a file "build.rs" with content "fn main() {}"
    And a file "src/main.rs" with content "fn main() {}"
    And a file "src/deep/lib.rs" with content "pub fn f() {}"
    When I glob "src/*.rs"
    Then the result should contain "src/main.rs"
    And the result should not contain "build.rs"
    And the result should not contain "deep/lib.rs"

  Scenario: Double star spans directories
    Given a file "build.rs" with content "fn main() {}"
    And a file "src/main.rs" with content "fn main() {}"
    And a file "src/deep/lib.rs" with content "pub fn f() {}"
    When I glob "src/**/*.rs"
    Then the result should contain "src/main.rs"
    And the result should contain "src/deep/lib.rs"
    And the result should not contain "build.rs"

  Scenario: A root-level pattern does not descend
    Given a file "build.rs" with content "fn main() {}"
    And a file "src/main.rs" with content "fn main() {}"
    When I glob "*.rs"
    Then the result should contain "build.rs"
    And the result should not contain "src/main.rs"

  Scenario: Question mark matches exactly one character
    Given a file "mod1.rs" with content "a"
    And a file "mod12.rs" with content "b"
    When I glob "mod?.rs"
    Then the result should contain "mod1.rs"
    And the result should not contain "mod12.rs"

  Scenario: Replace rewrites matching files
    Given a file "r2.txt" with content "old_text here"
    When I replace "old_text" with "new_text"
    Then the file "r2.txt" should contain "new_text"
    And the result field "total_replacements" should be 1

  # This used to assert `errors` = 1: the walk opened the link, read the target
  # through it, and only `write_replaced` refused at the last moment. The walk
  # now steps over links entirely — nothing outside the root can be read in the
  # first place — so the refusal never has to happen and `errors` is 0. The
  # write-side guard is still pinned directly by
  # `fsops::tests::write_replaced_refuses_to_replace_a_symlink`.
  # "old_text" -> "old_text!" keeps the link's target matching whichever order
  # the walk happens to visit the two entries in.
  Scenario: Replace never walks into a symlink
    Given a file "r1_real.txt" with content "old_text"
    And a symlink "r1.txt" pointing to "r1_real.txt"
    When I replace "old_text" with "old_text!"
    Then the result field "total_replacements" should be 1
    And the result field "errors" should be 0
    And the file "r1_real.txt" should contain "old_text!"
    And "r1.txt" should still be a symlink
