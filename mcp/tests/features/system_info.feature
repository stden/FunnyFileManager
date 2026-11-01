# Purpose: BDD scenarios for host introspection and AWS profile discovery.
Feature: Host and AWS profile introspection via MCP tools
  As an AI assistant
  I want to know where I am running and which cloud profiles exist
  So that I can pick the right target before touching anything

  # HOME is redirected to the scenario's temp directory, which makes the
  # home-derived answers below exact instead of machine-dependent.
  Background:
    Given a temporary directory

  Scenario: System info reports the home directory it was given
    When I get system info
    Then the result field "home_directory" should be the temporary directory

  Scenario: No credentials file means no profiles
    When I list AWS profiles
    Then the result field "count" should be 0

  Scenario: Profiles come from the credentials file in section order
    Given a file ".aws/credentials" with content "[dev]\naws_access_key_id = AKIAdev\n\n[prod]\naws_access_key_id = AKIAprod\n"
    When I list AWS profiles
    Then the result field "count" should be 2
    And the "profiles" entries should be named "dev, prod"

  # The config file names sections "[profile <name>]" while credentials names
  # them "[<name>]"; the region must land on the matching profile, not on
  # whichever one happened to be parsed last.
  Scenario: Regions from the config file attach to the right profile
    Given a file ".aws/credentials" with content "[dev]\naws_access_key_id = AKIAdev\n\n[prod]\naws_access_key_id = AKIAprod\n"
    And a file ".aws/config" with content "[profile dev]\nregion = eu-west-1\n\n[profile prod]\nregion = us-east-2\n"
    When I list AWS profiles
    Then profile 1 should be "dev" in region "eu-west-1"
    And profile 2 should be "prod" in region "us-east-2"

  Scenario: Drives include the S3 profiles found in the credentials file
    Given a file ".aws/credentials" with content "[staging]\naws_access_key_id = AKIAstaging\n"
    When I list drives
    Then the result should contain "staging"
    And the result should contain "s3"
