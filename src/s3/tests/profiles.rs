//! Purpose: unit tests for `list_aws_profiles` over a sandboxed `~/.aws/credentials`.

use super::*;

use std::fs;
use tempfile::TempDir;

#[test]
fn test_list_aws_profiles() {
    // This test just ensures the function doesn't panic
    let profiles = list_aws_profiles();
    // Should at least have "default"
    assert!(profiles.iter().any(|p| p.name == "default"));
}

#[test]
fn test_list_aws_profiles_mocked() {
    let temp_dir = TempDir::new().unwrap();
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir(&aws_dir).unwrap();
    let creds_file = aws_dir.join("credentials");
    fs::write(
        &creds_file,
        "[work]\nregion=us-west-2\n[personal]\nregion=us-east-1",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    assert!(profiles.iter().any(|p| p.name == "work"));
    assert!(profiles.iter().any(|p| p.name == "personal"));
    assert!(profiles.iter().any(|p| p.name == "default"));
}

#[test]
fn test_list_aws_profiles_reads_credentials() {
    let temp_dir = TempDir::new().expect("temp dir");
    let credentials_dir = temp_dir.path().join(".aws");
    fs::create_dir_all(&credentials_dir).expect("create .aws");
    fs::write(
        credentials_dir.join("credentials"),
        "[default]\naws_access_key_id=AKIA\n[team]\naws_secret_access_key=SECRET\n",
    )
    .expect("write credentials");
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    assert!(profiles.iter().any(|p| p.name == "default"));
    assert!(profiles.iter().any(|p| p.name == "team"));
}

#[test]
fn test_list_aws_profiles_deduplicates_default_section() {
    let temp_dir = TempDir::new().expect("test setup");
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir_all(&aws_dir).expect("test setup");
    fs::write(
        aws_dir.join("credentials"),
        "[default]\naws_access_key_id = a\n[prod]\naws_access_key_id = b\n",
    )
    .expect("test setup");

    let _guard = AwsHomeEnvGuard::set(temp_dir.path());
    let profiles = list_aws_profiles();
    let default_count = profiles.iter().filter(|p| p.name == "default").count();
    assert_eq!(default_count, 1);
    assert!(profiles.iter().any(|p| p.name == "prod"));
}

#[test]
fn test_list_aws_profiles_no_credentials_file() {
    let temp_dir = TempDir::new().expect("temp dir");
    // No .aws directory at all
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    // Should still have "default" as fallback
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "default");
}

#[test]
fn test_list_aws_profiles_empty_credentials_file() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir(&aws_dir).unwrap();
    fs::write(aws_dir.join("credentials"), "").unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    // Should have "default" inserted
    assert!(profiles.iter().any(|p| p.name == "default"));
}

#[test]
fn test_list_aws_profiles_default_already_present() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir(&aws_dir).unwrap();
    fs::write(
        aws_dir.join("credentials"),
        "[default]\naws_access_key_id=AKIA123\n[staging]\naws_access_key_id=AKIA456\n",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    // "default" should appear exactly once
    let default_count = profiles.iter().filter(|p| p.name == "default").count();
    assert_eq!(default_count, 1);
    assert!(profiles.iter().any(|p| p.name == "staging"));
}

#[test]
fn test_list_aws_profiles_malformed_lines() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir(&aws_dir).unwrap();
    fs::write(
        aws_dir.join("credentials"),
        "not a section\n[valid]\nkey=value\n[incomplete\nno-bracket]\n  [spaced]  \n",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    assert!(profiles.iter().any(|p| p.name == "valid"));
    // "[spaced]" after trim() becomes "[spaced]" which starts with [ and ends with ]
    assert!(profiles.iter().any(|p| p.name == "spaced"));
    // "[incomplete" doesn't end with ], should not appear
    assert!(!profiles.iter().any(|p| p.name == "incomplete"));
    // "no-bracket]" doesn't start with [, should not appear
    assert!(!profiles.iter().any(|p| p.name == "no-bracket"));
}

#[test]
fn test_list_aws_profiles_preserves_section_order_after_default() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir(&aws_dir).unwrap();
    fs::write(
        aws_dir.join("credentials"),
        "[work]\naws_access_key_id=1\n[personal]\naws_access_key_id=2\n",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profile_names: Vec<String> = list_aws_profiles()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(profile_names, vec!["default", "work", "personal"]);
}
