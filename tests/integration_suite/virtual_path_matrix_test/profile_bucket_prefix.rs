//! Purpose: Round-trips of `s3://profile@bucket/prefix` paths.

use super::*;

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
