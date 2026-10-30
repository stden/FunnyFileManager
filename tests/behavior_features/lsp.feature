Feature: Editor language services fail usefully and preserve responsive sessions
  Background:
    Given an isolated acceptance workspace

  Scenario: A missing component is explained and cached until an explicit retry
    Given a local missing-component language server fixture
    When I edit the Rust document "first.rs"
    Then the missing-component fixture has launched 0 times
    When I trust the fixture language server workspace
    And the editor reports the missing component
    Then the rendered screen contains "rustup component add rust-analyzer"
    And the missing-component fixture has launched 1 times
    When I close the editor before opening another document
    And I edit the Rust document "second.rs"
    And the editor reports the missing component
    Then the missing-component fixture has launched 1 times
    When I explicitly restart editor language services
    And the editor reports the missing component
    Then the missing-component fixture has launched 2 times

  Scenario: Typing remains available after the language server fails
    Given a local missing-component language server fixture
    When I edit the Rust document "first.rs"
    And I trust the fixture language server workspace
    And the editor reports the missing component
    And I dispatch the keys "x"
    Then the editor buffer equals "xfn main() {}"

  Scenario: A second document reuses its workspace server and shutdown is graceful
    Given a local ready language server fixture
    When I edit the Rust document "first.rs"
    And I trust the fixture language server workspace
    And the editor receives fixture diagnostics
    And I remember the language server process
    And I close the editor before opening another document
    And I edit the Rust document "second.rs"
    And the editor receives fixture diagnostics
    Then the same language server process is reused
    And the editor buffer equals "fn main() {}"
    And the fixture server has shut down cleanly
