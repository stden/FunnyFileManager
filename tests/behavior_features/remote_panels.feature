Feature: Remote SFTP and FTP panels browse view copy rename create folders and delete
  Background:
    Given an isolated acceptance workspace

  Scenario: SFTP remote panel navigates directories views files and leaves at root
    Given the left panel is connected to an "SFTP" host with remote files:
      | /etc/hostname     | edge-node-01 |
      | /var/log/app.log  | boot ok      |
    Then the left remote directory is "/"
    And the left remote listing equals ".. etc var"
    When I select remote entry "var" and open it
    And I select remote entry "log" and open it
    Then the left remote directory is "/var/log"
    And the left remote listing equals ".. app.log"
    When I select remote entry "app.log" and press "F3" and finish remote work
    Then the overlay is the viewer
    And the rendered screen contains "boot ok"
    When I dispatch the keys "Esc"
    And I select remote entry ".." and open it
    And I select remote entry ".." and open it
    And I select remote entry ".." and open it
    Then the left panel returns to local files

  Scenario: FTP remote panel creates folders renames copies to local and deletes files
    Given the left panel is connected to an "FTP" host at "/pub" with remote files:
      | /pub/readme.txt | release notes 2026 |
    When I dispatch the keys "F7"
    And I type the text "releases"
    And I dispatch the keys "Enter"
    And remote work finishes
    Then the left remote listing equals ".. readme.txt releases"
    When I select remote entry "readme.txt" and press "Shift+F6 Ctrl+A Backspace"
    And I type the text "changelog.txt"
    And I dispatch the keys "Enter"
    And remote work finishes
    Then the left remote listing equals ".. changelog.txt releases"
    When I select remote entry "changelog.txt" and press "F5 Enter" and finish remote work
    Then the right file "changelog.txt" equals "release notes 2026"
    When I select remote entry "changelog.txt" and press "F8 y" and finish remote work
    Then the left remote listing equals ".. releases"
