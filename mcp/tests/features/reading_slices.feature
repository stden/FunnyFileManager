Feature: File line slicing, text statistics, and punctuation inspection

  Background:
    Given a temporary directory

  Scenario: head_file returns the opening lines and reports whether EOF was reached
    Given a file "log.txt" with content "line 1\nline 2\nline 3\nline 4\nline 5\n"
    When I read the first 2 lines of "log.txt"
    Then the result field "lines_returned" should be 2
    And the result flag "reached_end" should be false
    And the sliced lines should be "line 1, line 2"

  Scenario: tail_file returns the trailing lines from the end of a file
    Given a file "log.txt" with content "line 1\nline 2\nline 3\nline 4\nline 5\n"
    When I read the last 2 lines of "log.txt"
    Then the result field "lines_returned" should be 2
    And the sliced lines should be "line 4, line 5"

  Scenario: read_lines extracts an inclusive 1-based range and rejects reversed bounds
    Given a file "log.txt" with content "line 1\nline 2\nline 3\nline 4\nline 5\n"
    When I read lines 2 to 4 of "log.txt"
    Then the result field "lines_returned" should be 3
    And the result flag "reached_end" should be false
    And the sliced lines should be "line 2, line 3, line 4"
    When I read lines 4 to 2 of "log.txt"
    Then the operation should fail with "before"

  Scenario: file_stats streams line, word, character, and longest-line counts
    Given a file "prose.txt" with content "alpha beta\ngamma\n"
    When I count text statistics of "prose.txt"
    Then the result field "lines" should be 2
    And the result field "words" should be 3
    And the result field "characters" should be 15
    And the result field "longest_line_chars" should be 10
    And the result field "longest_line_number" should be 1

  Scenario: text_format identifies LF line endings, trailing newline, and non-binary text
    Given a file "script.sh" with content "#!/bin/sh\necho ok\n"
    When I inspect text format of "script.sh"
    Then the result field "line_endings" should equal "lf"
    And the result field "lf_count" should be 2
    And the result flag "final_newline" should be true
    And the result field "byte_order_mark" should equal "none"
    And the result flag "binary" should be false
