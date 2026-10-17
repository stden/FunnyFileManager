//! List encoding, coexistence and cleanup without running a shell.

use super::*;
use crate::models::apply::{Names, Token, Tokens};

fn options(template: &str) -> ListOptions {
    match Tokens::new(template).next().expect("token") {
        Token::List(options) => options,
        other => panic!("list expected, got {other:?}"),
    }
}

#[test]
fn simultaneous_lists_have_distinct_names_and_encodings() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let panel = PanelContext {
        selected: vec![Names::same("файл с пробелами.txt")],
        ..PanelContext::default()
    };
    let mut lists = Lists::new(dir.path());
    let utf8 = lists.write(&panel, options("!@U!")).expect("UTF-8");
    let utf16 = lists.write(&panel, options("!@W!")).expect("UTF-16");
    assert_ne!(utf8, utf16);
    let text = format!(
        "файл с пробелами.txt{}",
        if cfg!(windows) { "\r\n" } else { "\n" }
    );
    assert_eq!(std::fs::read(&utf8).expect("UTF-8 file"), text.as_bytes());
    assert_eq!(
        std::fs::read(&utf16).expect("UTF-16 file"),
        text.encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>()
    );
    drop(lists);
    assert!(!Path::new(&utf8).exists());
    assert!(!Path::new(&utf16).exists());
}

#[test]
fn short_lists_use_the_filesystem_aliases() {
    let panel = PanelContext {
        selected: vec![Names {
            long: "long name.txt".into(),
            short: "LONGNA~1.TXT".into(),
        }],
        ..PanelContext::default()
    };
    assert_eq!(
        listing(&panel, options("!$Q!")).trim_end(),
        "\"LONGNA~1.TXT\""
    );
}

#[test]
fn native_code_pages_preserve_ascii_and_allow_empty_lists() {
    for encoding in [ListEncoding::Oem, ListEncoding::Ansi] {
        assert_eq!(
            encode("a.txt\r\n", encoding).expect("native encoding"),
            b"a.txt\r\n"
        );
        assert!(encode("", encoding).expect("empty").is_empty());
    }
}

#[test]
fn quoted_list_preserves_backslashes_and_dollars() {
    let panel = PanelContext {
        selected: vec![Names::same(r"dir\$name`1.txt")],
        ..PanelContext::default()
    };
    assert_eq!(
        listing(&panel, options("!@Q!")).trim_end(),
        r#""dir\$name`1.txt""#
    );
}
