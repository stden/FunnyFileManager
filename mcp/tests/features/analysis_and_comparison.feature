Feature: File and directory comparison, space analysis, duplicates, and PATH lookup

  Background:
    Given a temporary directory

  Scenario: compare_files distinguishes identical files from byte-offset differences
    Given a file "a.txt" with content "same-bytes"
    And a file "b.txt" with content "same-bytes"
    And a file "c.txt" with content "same-Xytes"
    When I compare file "a.txt" with file "b.txt"
    Then the result flag "identical" should be true
    When I compare file "a.txt" with file "c.txt"
    Then the result flag "identical" should be false
    And the result field "first_difference_at" should be 5

  Scenario: compare_directories reports left-only, right-only, and size-differing files
    Given a file "left_dir/only_left.txt" with content "left"
    And a file "left_dir/shared.txt" with content "short"
    And a file "right_dir/only_right.txt" with content "right"
    And a file "right_dir/shared.txt" with content "much longer"
    When I compare directory "left_dir" with directory "right_dir"
    Then the result flag "identical" should be false
    And the result list "only_in_left" should be "only_left.txt"
    And the result list "only_in_right" should be "only_right.txt"
    And the "differing_size" paths should be "shared.txt"

  Scenario: analyse_directory and largest_files rank subdirectories and files by size
    Given a file "big_sub/archive.zip" with content "0123456789abcdef"
    And a file "small_sub/note.md" with content "hi"
    When I analyse the temporary directory
    Then the result field "file_count" should be 2
    And the result field "total_size" should be 18
    And the "subdirectories" entries should be named "big_sub, small_sub"
    And the "largest_files" paths should be "big_sub/archive.zip, small_sub/note.md"
    When I list the top 1 largest files
    Then the result field "files_considered" should be 2
    And the "files" paths should be "big_sub/archive.zip"

  Scenario: recent_files and find_duplicates report modified files and duplicate groups
    Given a file "copy_one.txt" with content "duplicate payload"
    And a file "copy_two.txt" with content "duplicate payload"
    And a file "unique.txt" with content "distinct"
    When I list files modified within 1 days
    Then the result field "count" should be 3
    When I search for duplicate files
    Then the result field "duplicate_groups" should be 1
    And the result field "reclaimable" should be 17

  Scenario: which finds cargo on PATH and rejects directory components
    When I look up executable "cargo" on PATH
    Then the result flag "found" should be true
    When I look up executable "ffm-nonexistent-binary-xyz" on PATH
    Then the result flag "found" should be false
    When I look up executable "sub/cargo" on PATH
    Then the operation should fail with "bare executable"
