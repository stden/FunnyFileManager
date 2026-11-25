Feature: File panels highlight executable scripts and dirty Git repository badges
  Background:
    Given an isolated acceptance workspace

  Scenario: Shell Python and Windows command scripts render with executable styling
    Given the left panel contains files:
      | deploy.sh  |
      | runner.py  |
      | build.cmd  |
      | readme.txt |
    Then panel entry "deploy.sh" is marked executable and rendered in executable green
    And panel entry "runner.py" is marked executable and rendered in executable green
    And panel entry "build.cmd" is marked executable and rendered in executable green
    And panel entry "readme.txt" is rendered in ordinary file cyan

  Scenario: Background dirty-repo probes color only matching Git folders and clear when clean
    Given the left panel contains subdirectories:
      | dirty-svc |
      | clean-svc |
    When background Git probe marks left folder "dirty-svc" as dirty
    Then panel folder "dirty-svc" is rendered in dirty-repo amber
    And panel folder "clean-svc" is rendered in directory white
    And background Git probe for unlisted left folder "ghost-svc" is ignored
    When background Git probe marks left folder "dirty-svc" as clean
    Then panel folder "dirty-svc" is rendered in directory white
