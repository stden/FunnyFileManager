# Purpose: FFM's encoding switch in the editor on files whose bytes are not UTF-8. F8 toggles OEM-866 and Windows-1251, as Far's F8 toggles OEM and ANSI; Shift+F8 opens FFM's own encoding dialog, where Far's Shift+F8 opens a code page menu. hotkeys-and-parity.md records that difference. Saved files are checked as hex bytes written out here, not produced by the encoder under test.
Feature: FFM editor encoding switch on non-UTF-8 files

  Scenario: F8 toggles an OEM-866 file that opened as Windows-1251, keeps the caret, and F2 writes the bytes
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "OEM-866"
    When I open the TUI at 100 by 30
    And I press "Down F4"
    Then the screen contains "ЏаЁўҐв"
    And the screen does not contain "Привет"
    When I press "Right Right F8"
    Then the screen contains "Reloaded as OEM-866"
    And the screen contains "Привет"
    And the screen does not contain "Encoding (arrows select)"
    When I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "8F E0 21 A8 A2 A5 E2"

  Scenario: F8 twice toggles a Windows-1251 file to OEM-866 and back, and F2 writes the original bytes
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "Windows-1251"
    When I open the TUI at 100 by 30
    And I press "Down F4"
    Then the screen contains "Привет"
    When I press "F8"
    Then the screen contains "Reloaded as OEM-866"
    And the screen does not contain "Привет"
    When I press "F8"
    Then the screen contains "Reloaded as Windows-1251"
    And the screen contains "Привет"
    When I press "End"
    And I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "CF F0 E8 E2 E5 F2 21"

  Scenario: F8 does not reread the file over unsaved text
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "OEM-866"
    When I open the TUI at 100 by 30
    And I press "Down F4 End"
    And I type "!"
    And I press "F8"
    Then the screen contains "Save or discard changes before reloading the file"
    And the screen contains "ЏаЁўҐв!"

  Scenario: Shift+F8 opens the encoding dialog, and R reloads an OEM-866 file that opened as Windows-1251
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "OEM-866"
    When I open the TUI at 100 by 30
    And I press "Down F4"
    Then the screen contains "ЏаЁўҐв"
    And the screen does not contain "Привет"
    When I press "Shift+F8 Down R"
    Then the screen contains "Reloaded as OEM-866"
    And the screen contains "Привет"
    When I press "End"
    And I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "8F E0 A8 A2 A5 E2 21"

  Scenario: R in the encoding dialog reloads a Windows-1251 file as OEM-866 and back, and F2 writes the original bytes
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "Windows-1251"
    When I open the TUI at 100 by 30
    And I press "Down F4"
    Then the screen contains "Привет"
    When I press "Shift+F8 Down R"
    Then the screen contains "Reloaded as OEM-866"
    And the screen does not contain "Привет"
    When I press "Shift+F8 Up R"
    Then the screen contains "Reloaded as Windows-1251"
    And the screen contains "Привет"
    When I press "End"
    And I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "CF F0 E8 E2 E5 F2 21"

  Scenario: Enter in the encoding dialog sets the encoding that F2 writes, without re-reading the text
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "Windows-1251"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+F8 Down"
    Then the screen contains "Encoding (arrows select)"
    When I press "Enter"
    Then the screen contains "Encoding: OEM-866 (Shift+F8 changes encoding)"
    And the screen contains "Привет"
    When I press "End"
    And I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "8F E0 A8 A2 A5 E2 21"

  Scenario: Esc in the encoding dialog keeps the encoding that F2 writes
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "Windows-1251"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+F8 Down"
    Then the screen contains "Encoding (arrows select)"
    When I press "Esc"
    Then the screen does not contain "Encoding (arrows select)"
    And the screen contains "Привет"
    When I press "End"
    And I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "CF F0 E8 E2 E5 F2 21"

  Scenario: Shift+F8 opens the same encoding dialog
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "Windows-1251"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+F8"
    Then the screen contains "Encoding (arrows select)"
    And the screen contains "UTF-8 BOM"

  Scenario: Shift+F8 chooses UTF-8 BOM, and F2 writes the byte-order mark
    Given a TUI workspace
    And the left file "notes.txt" contains "Hi"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+F8 Down Enter End"
    And I type "!"
    And I press "F2"
    Then the left file "notes.txt" holds the bytes "EF BB BF 48 69 21"

  Scenario: F2 refuses text the chosen encoding cannot hold and leaves the file as it was
    Given a TUI workspace
    And the left file "notes.txt" is "Привет" in "Windows-1251"
    When I open the TUI at 100 by 30
    And I press "Down F4 Shift+F8 Down Enter End"
    And I type "€"
    And I press "F2"
    Then the screen contains "Cannot save: Text contains characters unavailable in OEM-866"
    And the left file "notes.txt" holds the bytes "CF F0 E8 E2 E5 F2"
