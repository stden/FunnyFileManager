//! Purpose: Local paths parse as local, never as S3.

use super::*;

fn assert_local_parse(path: &str) {
    let parsed = VirtualPath::parse(path);
    assert_eq!(parsed, VirtualPath::Local(PathBuf::from(path)));
    assert_eq!(parsed.display(), PathBuf::from(path).display().to_string());
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
