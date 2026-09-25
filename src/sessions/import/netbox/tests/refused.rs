//! Purpose: A whole file is refused — missing, oversized, not a `NetBox`
//! export, carrying a DTD, or past the node, depth or session limit.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::super::{import, parse, MAX_DEPTH, MAX_FILE_BYTES, MAX_NODES, MAX_SESSIONS};
use super::{sessions, FILE};
use crate::sessions::import::ImportError;
use crate::sessions::ReadError;

#[test]
fn import_refuses_a_missing_or_oversized_file_before_parsing_it() {
    let dir = TempDir::new().expect("temp dir");
    let missing = import(&dir.path().join("none.netbox")).expect_err("missing");
    assert!(matches!(missing, ImportError::Read(ReadError::Io(_))));
    let big = dir.path().join("big.netbox");
    let padding = " ".repeat(usize::try_from(MAX_FILE_BYTES).expect("fits"));
    fs::write(&big, format!("<NetBox>{padding}</NetBox>")).expect("write");
    assert!(matches!(
        import(&big),
        Err(ImportError::Read(ReadError::TooLarge {
            limit: MAX_FILE_BYTES
        }))
    ));
}

#[test]
fn a_document_that_is_not_a_netbox_export_is_refused() {
    let path = Path::new(FILE);
    assert!(matches!(
        parse("<WinSCP><Sessions/></WinSCP>", path),
        Err(ImportError::NotNetBox)
    ));
    assert!(matches!(parse("<NetBox>", path), Err(ImportError::Xml(_))));
    assert!(matches!(parse("", path), Err(ImportError::Xml(_))));
}

/// No DTD is accepted, so no entity is ever expanded or fetched: not an
/// internal one, not a billion laughs, not an external file.
#[test]
fn a_dtd_is_refused_so_no_entity_is_expanded_or_fetched() {
    let documents = [
        r#"<!DOCTYPE NetBox [<!ENTITY h "example.test">]><NetBox><Sessions><Session><HostName>&h;</HostName></Session></Sessions></NetBox>"#,
        r#"<!DOCTYPE NetBox [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;">]><NetBox>&c;</NetBox>"#,
        r#"<!DOCTYPE NetBox SYSTEM "file:///etc/hostname"><NetBox/>"#,
        r#"<!DOCTYPE NetBox [<!ENTITY x SYSTEM "file:///etc/hostname">]><NetBox>&x;</NetBox>"#,
    ];
    for document in documents {
        let outcome = parse(document, Path::new(FILE));
        assert!(
            matches!(
                &outcome,
                Err(ImportError::Xml(roxmltree::Error::DtdDetected))
            ),
            "{document}: {outcome:?}"
        );
    }
    assert!(matches!(
        parse("<NetBox>&undeclared;</NetBox>", Path::new(FILE)),
        Err(ImportError::Xml(_))
    ));
}

#[test]
fn a_tree_past_the_node_limit_is_refused() {
    let nodes = usize::try_from(MAX_NODES).expect("fits");
    let document = format!("<NetBox>{}</NetBox>", "<a/>".repeat(nodes));
    assert!(document.len() < usize::try_from(MAX_FILE_BYTES).expect("fits"));
    assert!(matches!(
        parse(&document, Path::new(FILE)),
        Err(ImportError::Xml(roxmltree::Error::NodesLimitReached))
    ));
}

#[test]
fn more_sessions_than_one_import_takes_are_refused_and_the_limit_is_not() {
    let entry = "<Session><HostName>h.test</HostName></Session>";
    let at_limit = sessions(&entry.repeat(MAX_SESSIONS));
    let imported = parse(&at_limit, Path::new(FILE)).expect("at the limit");
    assert_eq!(imported.sessions.len(), 1, "the same name each time");
    assert_eq!(imported.skipped.len(), MAX_SESSIONS - 1);
    let over = sessions(&entry.repeat(MAX_SESSIONS + 1));
    assert!(matches!(
        parse(&over, Path::new(FILE)),
        Err(ImportError::TooManySessions {
            limit: MAX_SESSIONS
        })
    ));
}

/// The guest sweep's hostile fixture: 20 000 nested elements fit every other
/// limit, and the parser recursed once per level until the stack overflowed
/// and aborted FFM with the terminal still raw. The depth is refused before
/// the parser runs.
#[test]
fn a_tree_nested_past_the_depth_limit_is_refused_before_it_is_parsed() {
    let levels = 20_000;
    let document = sessions(&format!(
        "{}{}",
        "<a>".repeat(levels),
        "</a>".repeat(levels)
    ));
    assert!(document.len() < usize::try_from(MAX_FILE_BYTES).expect("fits"));
    assert!(levels < usize::try_from(MAX_NODES).expect("fits"));
    assert!(matches!(
        parse(&document, Path::new(FILE)),
        Err(ImportError::TooDeep { limit: MAX_DEPTH })
    ));
}
