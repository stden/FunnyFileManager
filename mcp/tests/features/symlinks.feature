# Purpose: BDD scenarios for symlink-safe delete and inspection.
@unix
Feature: Symlink handling via MCP tools
  As an MCP server
  I want to operate on links themselves
  So that a link never stands in for the data it points at

  Background:
    Given a temporary directory

  Scenario: Deleting a link to a directory keeps the target
    Given a subdirectory "data/tree"
    And a file "data/tree/keep.txt" with content "keep"
    And a symlink "link_to_tree" pointing to "data/tree"
    When I delete "link_to_tree"
    Then the symlink "link_to_tree" should not exist
    And the file "data/tree/keep.txt" should exist

  Scenario: Deleting a dangling link succeeds
    Given a symlink "dangling" pointing to "missing_target"
    When I delete "dangling"
    Then the symlink "dangling" should not exist

  Scenario: File info reports a link as a symlink
    Given a file "target.txt" with content "hello"
    And a symlink "alias.txt" pointing to "target.txt"
    When I get info for "alias.txt"
    Then the result should contain "symlink"
    And the result should contain "symlink_target"

  Scenario: A created link points at the target and leaves it untouched
    Given a file "real.txt" with content "payload"
    When I link "alias.txt" to "real.txt"
    Then the file "alias.txt" should contain "payload"
    And the file "real.txt" should contain "payload"

  # `read_symlink` must not resolve the final component: it reports where the
  # link points, which is exactly what a `get_file_info` on the target cannot
  # tell you.
  Scenario: Reading a link reports its target
    Given a file "real.txt" with content "payload"
    And a symlink "alias.txt" pointing to "real.txt"
    When I read the link "alias.txt"
    Then the result should contain "real.txt"
    And the result flag "target_exists" should be true

  Scenario: Reading a dangling link still reports the missing target
    Given a symlink "broken" pointing to "gone.txt"
    When I read the link "broken"
    Then the result should contain "gone.txt"
    And the result flag "target_exists" should be false

  Scenario: Reading a plain file as a link fails
    Given a file "plain.txt" with content "not a link"
    When I read the link "plain.txt"
    Then the operation should fail

  # The batch tools have to be exactly as link-safe as the single ones. This
  # one used to canonicalize the path first, so it handed `remove_dir_all` the
  # directory the link pointed at and left the link dangling over the wreckage.
  Scenario: Batch deleting a link to a directory keeps the target
    Given a subdirectory "data"
    And a file "data/keep.txt" with content "keep"
    And a symlink "link" pointing to "data"
    When I batch delete "link"
    Then the result field "total_deleted" should be 1
    And the result field "total_errors" should be 0
    And the symlink "link" should not exist
    And the directory "data" should exist
    And the file "data/keep.txt" should contain "keep"

  # Same resolution bug on the move side: it renamed the real file away and
  # reported the *target's* name as the thing that moved.
  Scenario: Batch moving a link moves the link, not its target
    Given a file "real.txt" with content "payload"
    And a symlink "alias.txt" pointing to "real.txt"
    When I batch move "alias.txt" into "moved"
    Then the result list "moved" should be "alias.txt"
    And the file "real.txt" should contain "payload"
    And the symlink "alias.txt" should not exist
    And "moved/alias.txt" should still be a symlink

  Scenario: Batch copying a link copies the link, not the file it points at
    Given a file "real.txt" with content "payload"
    And a symlink "alias.txt" pointing to "real.txt"
    When I batch copy "alias.txt" into "archive"
    Then the result list "copied" should be "alias.txt"
    And "alias.txt" should still be a symlink
    And "archive/alias.txt" should still be a symlink

  # `safe_path` confines the directory a walk starts from, not the entries it
  # reaches. A link planted inside the root once handed `grep` the contents of
  # a file the caller could never have opened by name.
  Scenario: Grep does not read through a link that leaves the root
    Given a file "secret.txt" with content "aws_secret_access_key" outside the root
    And a symlink "creds" pointing outside the root at "secret.txt"
    And a file "inside.txt" with content "nothing to see"
    When I grep for "aws_secret_access_key"
    Then the result field "count" should be 0

  Scenario: Replace does not read through a link that leaves the root
    Given a file "secret.txt" with content "aws_secret_access_key" outside the root
    And a symlink "creds" pointing outside the root at "secret.txt"
    When I preview replacing "aws_secret_access_key" with "REDACTED"
    Then the result field "files_affected" should be 0
    And the result field "total_replacements" should be 0

  Scenario: Glob does not report a link that leaves the root
    Given a file "secret.txt" with content "top secret" outside the root
    And a symlink "creds" pointing outside the root at "secret.txt"
    And a file "inside.txt" with content "public"
    When I glob "*"
    Then the result field "count" should be 1
    And the result should not contain "creds"

  Scenario: Deleting by pattern leaves a matching link alone
    Given a file "secret.txt" with content "top secret" outside the root
    And a symlink "creds.tmp" pointing outside the root at "secret.txt"
    When I delete files matching "*.tmp"
    Then the result field "deleted_count" should be 0
    And "creds.tmp" should still be a symlink

  Scenario: Clearing the readonly flag keeps a private file private
    Given a file "key.txt" with content "secret"
    And the file "key.txt" has mode "600"
    When I clear the readonly flag on "key.txt"
    Then the result should contain "100600"

  Scenario: Listing does not classify an out-of-root directory link as a dir
    Given a symlink "escape_dir" pointing outside the root at "."
    And a file "aaa.txt" with content "local"
    When I list the temporary directory
    Then "aaa.txt" should appear before "escape_dir" in entries

  Scenario: Sorting does not classify an out-of-root directory link as a dir
    Given a symlink "escape_dir" pointing outside the root at "."
    And a file "aaa.txt" with content "local"
    When I sort the temporary directory by "name"
    Then "aaa.txt" should appear before "escape_dir" in entries
