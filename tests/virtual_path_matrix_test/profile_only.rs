//! Purpose: Round-trips of `s3://profile@` paths that name only a profile.

use super::*;

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
