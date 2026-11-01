Feature: Images can be inspected without leaving the file manager
  Background:
    Given an isolated acceptance workspace

  Scenario: F3 displays image details and zoom controls
    Given a PNG image named "photo.png" with size 2 by 1
    When I dispatch the keys "F3"
    And the image preview finishes loading
    Then the overlay is the viewer
    And the rendered screen contains "photo.png"
    And the rendered screen contains "PNG"
    And the rendered screen contains "2×1"
    And the rendered screen contains "0 Fit"
    And the image is rendered in its original color
    When I dispatch the keys "+"
    Then the rendered screen contains "2x"
    When I dispatch the keys "0"
    Then the rendered screen contains "1x"
    When I dispatch the keys "Esc"
    Then the overlay is closed
    And the left panel cursor selects "photo.png"

  Scenario: F3 closes an open image viewer
    Given a PNG image named "photo.png" with size 2 by 1
    When I dispatch the keys "F3 F3"
    Then the overlay is closed
    And the left panel cursor selects "photo.png"

  Scenario: A corrupt image shows an error and returns to the file panel
    Given a corrupt PNG image named "broken.png"
    When I dispatch the keys "F3"
    And the image preview finishes loading
    Then the rendered screen contains "Cannot show image:"
    And the rendered screen contains "Enter to open externally"
    When I dispatch the keys "Esc"
    Then the overlay is closed
    And the left panel cursor selects "broken.png"

  Scenario: Quick view follows the selection and restores the other panel
    Given a PNG image named "first.png" with size 2 by 1
    And a PNG image named "second.png" with size 3 by 2
    When I dispatch the keys "Ctrl+Q"
    And the image preview finishes loading
    Then the right quick view shows "second.png" as a 3 by 2 image
    When I dispatch the keys "Up"
    And the image preview finishes loading
    Then the right quick view shows "first.png" as a 2 by 1 image
    When I dispatch the keys "Ctrl+Q"
    Then the right panel is a file list
    And the left panel cursor selects "first.png"
