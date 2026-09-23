#![cfg(feature = "s3")]
//! Purpose: Checks S3 virtual path parsing across a matrix of valid forms.
//!
use funny_file_manager::s3::VirtualPath;
use std::path::PathBuf;

fn assert_profile_only_roundtrip(profile: &str) {
    let original = VirtualPath::S3 {
        profile: profile.to_owned(),
        bucket: None,
        prefix: String::new(),
    };
    let displayed = original.display();
    let reparsed = VirtualPath::parse(&displayed);
    assert_eq!(reparsed, original, "displayed path: {displayed}");
}

fn assert_profile_bucket_roundtrip(profile: &str, bucket: &str) {
    let original = VirtualPath::S3 {
        profile: profile.to_owned(),
        bucket: Some(bucket.to_owned()),
        prefix: String::new(),
    };
    let displayed = original.display();
    let reparsed = VirtualPath::parse(&displayed);
    assert_eq!(reparsed, original, "displayed path: {displayed}");
}

fn assert_profile_bucket_prefix_roundtrip(profile: &str, bucket: &str, prefix: &str) {
    let original = VirtualPath::S3 {
        profile: profile.to_owned(),
        bucket: Some(bucket.to_owned()),
        prefix: prefix.to_owned(),
    };
    let displayed = original.display();
    let reparsed = VirtualPath::parse(&displayed);
    assert_eq!(reparsed, original, "displayed path: {displayed}");
}

fn assert_local_parse(path: &str) {
    let parsed = VirtualPath::parse(path);
    assert_eq!(parsed, VirtualPath::Local(PathBuf::from(path)));
    assert_eq!(parsed.display(), PathBuf::from(path).display().to_string());
}

macro_rules! profile_only_tests {
    ($(($name:ident, $profile:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_profile_only_roundtrip($profile);
            }
        )+
    };
}

macro_rules! profile_bucket_tests {
    ($(($name:ident, $profile:expr, $bucket:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_profile_bucket_roundtrip($profile, $bucket);
            }
        )+
    };
}

macro_rules! profile_bucket_prefix_tests {
    ($(($name:ident, $profile:expr, $bucket:expr, $prefix:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_profile_bucket_prefix_roundtrip($profile, $bucket, $prefix);
            }
        )+
    };
}

macro_rules! local_path_tests {
    ($(($name:ident, $path:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_local_parse($path);
            }
        )+
    };
}

profile_only_tests!(
    (test_profile_only_001, "default"),
    (test_profile_only_002, "prod"),
    (test_profile_only_003, "staging"),
    (test_profile_only_004, "dev"),
    (test_profile_only_005, "ops"),
    (test_profile_only_006, "backup"),
    (test_profile_only_007, "media"),
    (test_profile_only_008, "analytics"),
    (test_profile_only_009, "qa"),
    (test_profile_only_010, "uat"),
    (test_profile_only_011, "team1"),
    (test_profile_only_012, "team2"),
    (test_profile_only_013, "team3"),
    (test_profile_only_014, "blue"),
    (test_profile_only_015, "green"),
    (test_profile_only_016, "north"),
    (test_profile_only_017, "south"),
    (test_profile_only_018, "east"),
    (test_profile_only_019, "west"),
    (test_profile_only_020, "archive"),
    (test_profile_only_021, "logs"),
    (test_profile_only_022, "billing"),
    (test_profile_only_023, "platform"),
    (test_profile_only_024, "search"),
    (test_profile_only_025, "infra")
);

profile_bucket_tests!(
    (test_profile_bucket_001, "default", "bucket-a"),
    (test_profile_bucket_002, "prod", "bucket-b"),
    (test_profile_bucket_003, "staging", "bucket-c"),
    (test_profile_bucket_004, "dev", "bucket-d"),
    (test_profile_bucket_005, "ops", "bucket-e"),
    (test_profile_bucket_006, "backup", "bucket-f"),
    (test_profile_bucket_007, "media", "bucket-g"),
    (test_profile_bucket_008, "analytics", "bucket-h"),
    (test_profile_bucket_009, "qa", "bucket-i"),
    (test_profile_bucket_010, "uat", "bucket-j"),
    (test_profile_bucket_011, "team1", "bucket-k"),
    (test_profile_bucket_012, "team2", "bucket-l"),
    (test_profile_bucket_013, "team3", "bucket-m"),
    (test_profile_bucket_014, "blue", "bucket-n"),
    (test_profile_bucket_015, "green", "bucket-o"),
    (test_profile_bucket_016, "north", "bucket-p"),
    (test_profile_bucket_017, "south", "bucket-q"),
    (test_profile_bucket_018, "east", "bucket-r"),
    (test_profile_bucket_019, "west", "bucket-s"),
    (test_profile_bucket_020, "archive", "bucket-t"),
    (test_profile_bucket_021, "logs", "bucket-u"),
    (test_profile_bucket_022, "billing", "bucket-v"),
    (test_profile_bucket_023, "platform", "bucket-w"),
    (test_profile_bucket_024, "search", "bucket-x"),
    (test_profile_bucket_025, "infra", "bucket-y")
);

profile_bucket_prefix_tests!(
    (
        test_profile_bucket_prefix_001,
        "default",
        "bucket-a",
        "one/two"
    ),
    (
        test_profile_bucket_prefix_002,
        "prod",
        "bucket-b",
        "logs/2024"
    ),
    (
        test_profile_bucket_prefix_003,
        "staging",
        "bucket-c",
        "logs/2025/01"
    ),
    (
        test_profile_bucket_prefix_004,
        "dev",
        "bucket-d",
        "images/raw"
    ),
    (
        test_profile_bucket_prefix_005,
        "ops",
        "bucket-e",
        "images/final"
    ),
    (
        test_profile_bucket_prefix_006,
        "backup",
        "bucket-f",
        "db/snapshots"
    ),
    (
        test_profile_bucket_prefix_007,
        "media",
        "bucket-g",
        "audio/tracks"
    ),
    (
        test_profile_bucket_prefix_008,
        "analytics",
        "bucket-h",
        "warehouse/facts"
    ),
    (
        test_profile_bucket_prefix_009,
        "qa",
        "bucket-i",
        "test/cases"
    ),
    (
        test_profile_bucket_prefix_010,
        "uat",
        "bucket-j",
        "test/results"
    ),
    (
        test_profile_bucket_prefix_011,
        "team1",
        "bucket-k",
        "alpha/beta"
    ),
    (
        test_profile_bucket_prefix_012,
        "team2",
        "bucket-l",
        "gamma/delta"
    ),
    (
        test_profile_bucket_prefix_013,
        "team3",
        "bucket-m",
        "epsilon/zeta"
    ),
    (
        test_profile_bucket_prefix_014,
        "blue",
        "bucket-n",
        "canary/releases"
    ),
    (
        test_profile_bucket_prefix_015,
        "green",
        "bucket-o",
        "stable/releases"
    ),
    (
        test_profile_bucket_prefix_016,
        "north",
        "bucket-p",
        "region/eu"
    ),
    (
        test_profile_bucket_prefix_017,
        "south",
        "bucket-q",
        "region/us"
    ),
    (
        test_profile_bucket_prefix_018,
        "east",
        "bucket-r",
        "region/apac"
    ),
    (
        test_profile_bucket_prefix_019,
        "west",
        "bucket-s",
        "region/latam"
    ),
    (
        test_profile_bucket_prefix_020,
        "archive",
        "bucket-t",
        "cold/storage"
    ),
    (
        test_profile_bucket_prefix_021,
        "logs",
        "bucket-u",
        "service/auth"
    ),
    (
        test_profile_bucket_prefix_022,
        "billing",
        "bucket-v",
        "monthly/2026-03"
    ),
    (
        test_profile_bucket_prefix_023,
        "platform",
        "bucket-w",
        "clusters/primary"
    ),
    (
        test_profile_bucket_prefix_024,
        "search",
        "bucket-x",
        "indexes/live"
    ),
    (
        test_profile_bucket_prefix_025,
        "infra",
        "bucket-y",
        "terraform/state"
    )
);

local_path_tests!(
    (test_local_path_001, "/tmp"),
    (test_local_path_002, "/var/log"),
    (test_local_path_003, "/usr/local/bin"),
    (test_local_path_004, "/opt/data"),
    (test_local_path_005, "/srv/www"),
    (test_local_path_006, "./relative/path"),
    (test_local_path_007, "../parent/path"),
    (test_local_path_008, "docs/readme.md"),
    (test_local_path_009, "data/input.csv"),
    (test_local_path_010, "logs/app.log"),
    (test_local_path_011, "config/dev.toml"),
    (test_local_path_012, "config/prod.toml"),
    (test_local_path_013, "nested/deep/tree/file.txt"),
    (test_local_path_014, "single"),
    (test_local_path_015, "path-with-dash"),
    (test_local_path_016, "path_with_underscore"),
    (test_local_path_017, "mixed/path-01/file_02.txt"),
    (test_local_path_018, "./a"),
    (test_local_path_019, "./b/c"),
    (test_local_path_020, "./d/e/f"),
    (test_local_path_021, "tmp/cache"),
    (test_local_path_022, "tmp/cache/index"),
    (test_local_path_023, "assets/images/logo.png"),
    (test_local_path_024, "assets/styles/site.css"),
    (test_local_path_025, "scripts/run.sh")
);
