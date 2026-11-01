//! Purpose: The text a JSON rewrite keeps: key order, the byte-order mark,
//! every other value's exact text, the line ends and the indentation.

use super::*;

#[test]
fn other_servers_and_top_level_keys_survive_in_order() {
    let text = r#"{
  "theme": "dark",
  "mcpServers": {
    "zeta": {"command": "z"},
    "alpha": {"command": "a"}
  },
  "after": 1
}"#;
    let written = upserted(text, &all_projects());
    let order = [
        "\"theme\"",
        "\"mcpServers\"",
        "\"zeta\"",
        "\"alpha\"",
        "\"ffm-mcp\"",
        "\"after\"",
    ];
    let positions: Vec<usize> = order
        .iter()
        .map(|key| written.find(key).unwrap_or_else(|| panic!("{key} lost")))
        .collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted, "{written}");
}

#[test]
fn a_bom_is_kept() {
    let written = upserted("\u{feff}{}", &all_projects());
    assert!(written.starts_with('\u{feff}'));
    assert!(written.ends_with("}\n"));
    assert!(!upserted("{}", &all_projects()).starts_with('\u{feff}'));
}

/// Review finding: a `Value` round trip rewrote other people's numbers
/// (`1e3` became `1000.0`, a 24-digit integer lost its tail) and turned the
/// file's CRLF line ends into LF.
#[test]
fn every_other_value_keeps_its_exact_text_and_the_line_ends() {
    let text = "{\r\n  \"big\": 123456789012345678901234,\r\n  \"exp\": 1e3,\r\n  \
                \"pi\": 3.14159265358979323846,\r\n  \"nested\": {\"a\":1,  \"b\":[1e-7]},\r\n  \
                \"mcpServers\": {\r\n    \"other\": {\"command\":\"x\",\"n\":2E+2}\r\n  }\r\n}\r\n";
    let written = upserted(text, &all_projects());
    for kept in [
        "\"big\": 123456789012345678901234,",
        "\"exp\": 1e3,",
        "\"pi\": 3.14159265358979323846,",
        "\"nested\": {\"a\":1,  \"b\":[1e-7]},",
        "\"other\": {\"command\":\"x\",\"n\":2E+2},",
    ] {
        assert!(written.contains(kept), "lost {kept:?}:\n{written}");
    }
    assert!(!written.replace("\r\n", "").contains('\n'), "{written:?}");
    assert!(written.ends_with("}\r\n"), "{written:?}");
    assert_eq!(
        json_state(&written, &all_projects()),
        ConfigState::Registered
    );
    assert_eq!(upserted(&written, &all_projects()), written);
}

/// The kept values keep their lines, so the rewrite indents as the file does.
#[test]
fn a_file_indented_by_four_keeps_four() {
    let text = "{\n    \"theme\": {\n        \"a\": 1\n    },\n    \"mcpServers\": {\n        \"other\": {\n            \"command\": \"x\"\n        }\n    }\n}\n";
    let expected = format!(
        "{{\n    \"theme\": {{\n        \"a\": 1\n    }},\n    \"mcpServers\": {{\n        \"other\": {{\n            \"command\": \"x\"\n        }},\n        \"ffm-mcp\": {{\n            \"command\": {:?},\n            \"args\": [],\n            \"env\": {{}}\n        }}\n    }}\n}}\n",
        server().as_str()
    );
    assert_eq!(upserted(text, &all_projects()), expected);
}
