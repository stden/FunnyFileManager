//! Purpose: Round-trips of `s3://profile@bucket` paths.

use super::*;

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
