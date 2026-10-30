//! Local fixtures for profile ordering and optional config metadata.

use std::fs;

use serde_json::json;
use tempfile::TempDir;

use super::{read_names, read_profiles};

#[test]
fn credentials_define_order_and_config_merges_regions_into_duplicates() {
    let directory = TempDir::new().unwrap();
    fs::write(
        directory.path().join("credentials"),
        "[work]\naws_secret_access_key = secret\n[default]\n[work]\n[offline]\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("config"),
        "[profile work]\nregion = eu-west-1\nregional_endpoints = legacy\n[default]\nregion=us-east-1\n\
         [sso-session work]\nsso_region = ap-southeast-2\nregion = ap-southeast-2\n\
         [profile config-only]\nregion = ap-south-1\n\
         [profile work]\nregion = eu-north-1\n",
    )
    .unwrap();

    assert_eq!(
        read_names(directory.path()),
        ["work", "default", "work", "offline"]
    );
    assert_eq!(
        read_profiles(directory.path()),
        vec![
            json!({ "name": "work", "region": "eu-north-1" }),
            json!({ "name": "default", "region": "us-east-1" }),
            json!({ "name": "work", "region": "eu-north-1" }),
            json!({ "name": "offline" }),
        ]
    );
}

#[test]
fn regional_endpoints_and_sso_session_do_not_clobber_profile_region() {
    let directory = TempDir::new().unwrap();
    fs::write(directory.path().join("credentials"), "[work]\n").unwrap();
    fs::write(
        directory.path().join("config"),
        "[profile work]\nregion = eu-west-1\nregional_endpoints = legacy\n[sso-session work]\nregion = us-west-2\n",
    )
    .unwrap();
    assert_eq!(
        read_profiles(directory.path()),
        vec![json!({ "name": "work", "region": "eu-west-1" })]
    );
}

#[test]
fn missing_or_unreadable_files_keep_listing_best_effort() {
    let directory = TempDir::new().unwrap();
    assert_eq!(read_profiles(directory.path()).len(), 0);
    fs::write(directory.path().join("credentials"), " [default] \n").unwrap();
    fs::create_dir(directory.path().join("config")).unwrap();
    assert_eq!(
        read_profiles(directory.path()),
        vec![json!({ "name": "default" })]
    );
    fs::remove_file(directory.path().join("credentials")).unwrap();
    fs::create_dir(directory.path().join("credentials")).unwrap();
    assert_eq!(read_profiles(directory.path()).len(), 0);
}
