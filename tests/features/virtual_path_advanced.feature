# Purpose: BDD scenarios for virtual path parsing and navigation.
Feature: Advanced VirtualPath parsing
  As a developer
  I want VirtualPath to correctly parse all path formats
  So that local and S3 paths are never confused

  Scenario: Parse absolute local path
    When I parse path "/home/user/docs"
    Then the path type should be Local

  Scenario: Parse relative local path
    When I parse path "relative/path"
    Then the path type should be Local

  Scenario: Parse S3 path with bucket only
    When I parse path "s3://mybucket"
    Then the path type should be S3
    And the bucket should be "mybucket"
    And the prefix should be ""

  Scenario: Parse S3 path with bucket and prefix
    When I parse path "s3://mybucket/some/prefix/"
    Then the path type should be S3
    And the bucket should be "mybucket"
    And the prefix should be "some/prefix/"

  Scenario: Parse S3 path with named profile
    When I parse path "s3://prod@mybucket/data"
    Then the path type should be S3
    And the profile should be "prod"
    And the bucket should be "mybucket"
    And the prefix should be "data"

  Scenario: Parse empty S3 path
    When I parse path "s3://"
    Then the path type should be S3
    And the bucket should be empty

  Scenario: Parse S3 profile without bucket
    When I parse path "s3://myprofile@"
    Then the path type should be S3
    And the profile should be "myprofile"

  Scenario: Parse dot path as local
    When I parse path "."
    Then the path type should be Local

  Scenario: Parse home directory path as local
    When I parse path "~"
    Then the path type should be Local

  Scenario: S3 display roundtrip with bucket and prefix
    When I parse path "s3://bucket/folder/"
    Then the display should be "s3://default@bucket/folder/"
