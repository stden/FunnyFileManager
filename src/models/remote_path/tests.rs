//! Unit tests: a remote path is absolute and normalised, joins only single
//! names, climbs to the root and no further, and refuses what a user or a
//! file must not smuggle in.

use super::{is_name, RemotePath, MAX_PATH_BYTES};
use crate::models::{Field, InvalidField, Problem};

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("a valid remote path")
}

fn refusal(text: &str) -> Problem {
    let error = RemotePath::parse(text).expect_err("refused");
    assert_eq!(error.field, Field::RemoteDir);
    error.problem
}

#[test]
fn a_path_is_normalised_once() {
    assert_eq!(path("/var//www/").as_str(), "/var/www");
    assert_eq!(path("  /srv/x  ").as_str(), "/srv/x");
    assert_eq!(path("/").as_str(), "/");
    assert_eq!(path("///").to_string(), "/");
    assert!(path("//").is_root());
    assert!(!path("/a").is_root());
}

#[test]
fn a_path_that_is_not_absolute_or_clean_is_refused() {
    assert_eq!(refusal(""), Problem::Empty);
    assert_eq!(refusal("   "), Problem::Empty);
    assert_eq!(refusal("var/www"), Problem::NotAbsolute);
    assert_eq!(refusal("~/www"), Problem::NotAbsolute);
    assert_eq!(refusal("/var/../etc"), Problem::DotSegment);
    assert_eq!(refusal("/var/./www"), Problem::DotSegment);
    assert_eq!(refusal("/var/\u{1b}[2J"), Problem::ControlCharacter);
    assert_eq!(refusal("/a\0b"), Problem::ControlCharacter);
}

#[test]
fn a_path_longer_than_path_max_is_refused_and_one_at_it_is_not() {
    let longest = format!("/{}", "a".repeat(MAX_PATH_BYTES - 1));
    assert_eq!(path(&longest).as_str().len(), MAX_PATH_BYTES);
    let longer = format!("{longest}a");
    assert_eq!(
        refusal(&longer),
        Problem::TooLong {
            limit: MAX_PATH_BYTES
        }
    );
}

#[test]
fn join_takes_one_name_and_nothing_that_would_move_the_panel_elsewhere() {
    let root = RemotePath::root();
    assert_eq!(root.join("etc").map(String::from).as_deref(), Some("/etc"));
    assert_eq!(
        path("/var").join("www").map(String::from).as_deref(),
        Some("/var/www")
    );
    for bad in ["", ".", "..", "a/b", "/", "a\0"] {
        assert_eq!(root.join(bad), None, "{bad:?}");
        assert!(!is_name(bad), "{bad:?}");
    }
    assert!(is_name("..."), "only exactly `.` and `..` are special");
    assert!(is_name("\u{1b}"), "a host's odd name is still one name");
}

#[test]
fn parent_climbs_to_the_root_and_stops() {
    assert_eq!(path("/var/www").parent(), Some(path("/var")));
    assert_eq!(path("/var").parent(), Some(RemotePath::root()));
    assert_eq!(RemotePath::root().parent(), None);
}

#[test]
fn serde_goes_through_the_check() {
    let json = serde_json::to_string(&path("/var/www")).expect("serialise");
    assert_eq!(json, "\"/var/www\"");
    assert_eq!(
        serde_json::from_str::<RemotePath>("\"/srv//x/\"").expect("parse"),
        path("/srv/x")
    );
    let refused = serde_json::from_str::<RemotePath>("\"relative\"").expect_err("refused");
    assert!(
        refused
            .to_string()
            .contains(&InvalidField::new(Field::RemoteDir, Problem::NotAbsolute).to_string()),
        "{refused}"
    );
    assert_eq!(
        RemotePath::try_from("/a/".to_owned()).map(String::from),
        Ok("/a".to_owned())
    );
}
