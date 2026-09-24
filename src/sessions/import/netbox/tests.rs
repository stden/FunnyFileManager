//! Unit tests: a `.netbox` export becomes sessions with every value checked;
//! a DTD, an entity, an oversized file or tree, or a wrong root is refused
//! outright; a bad entry is skipped with its reason; a saved password is
//! counted and never read. Every fixture is made up and written to a
//! temporary directory.

use std::path::Path;

use super::parse;
use crate::models::Session;
use crate::sessions::import::{Imported, Skipped};

mod decoding;
mod entries;
mod refused;
mod skipped;

const FILE: &str = "/srv/x/demo.netbox";

fn sessions(body: &str) -> String {
    format!("<NetBox><Sessions>{body}</Sessions></NetBox>")
}

fn parsed(xml: &str) -> Imported {
    parse(xml, Path::new(FILE)).expect("a NetBox export")
}

fn only_session(xml: &str) -> Session {
    let imported = parsed(xml);
    assert_eq!(imported.skipped, [], "nothing skipped");
    let [session] = imported.sessions.as_slice() else {
        panic!("one session: {:?}", imported.sessions);
    };
    session.clone()
}

fn only_skip(xml: &str) -> Skipped {
    let imported = parsed(xml);
    assert_eq!(imported.sessions, [], "nothing imported");
    let [skipped] = imported.skipped.as_slice() else {
        panic!("one skipped entry: {:?}", imported.skipped);
    };
    skipped.clone()
}
