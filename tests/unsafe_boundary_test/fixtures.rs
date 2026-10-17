//! Purpose: The checker on planted sources: what it finds, permits and rejects.

use super::*;

#[test]
fn unsafe_in_a_models_file_fails_even_with_its_allow() {
    let source = "#[allow(unsafe_code)]\nfn read() -> u8 {\n    unsafe { *PTR }\n}\n";
    let found = violations("src/models/fixture.rs", &sites_in("fixture", source));
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("src/models/fixture.rs:1: allow(unsafe_code)"));
    assert!(found[1].starts_with("src/models/fixture.rs:3: unsafe block"));
    assert!(found.iter().all(|message| message.contains("forbidden")));
}

#[test]
fn an_allowlisted_boundary_passes() {
    let source = "#[allow(unsafe_code)]\nfn drives() -> u32 {\n    unsafe { mask() }\n}\n";
    let sites = sites_in("fixture", source);
    assert_eq!(sites.len(), 2, "{sites:?}");
    assert!(violations("src/platform/windows.rs", &sites).is_empty());
}

#[test]
fn an_allow_outside_the_allowlist_fails() {
    let sites = sites_in("fixture", "#[allow(unsafe_code)]\nfn nothing_yet() {}\n");
    let found = violations("src/utils.rs", &sites);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("allow(unsafe_code)"), "{found:?}");
    assert!(found[0].contains("not a reviewed"), "{found:?}");
}

#[test]
fn a_file_without_unsafe_passes_anywhere() {
    let sites = sites_in("fixture", "fn safe() -> u8 { 1 }\n");
    assert!(violations("src/models/fixture.rs", &sites).is_empty());
    assert!(violations("mcp/src/main.rs", &sites).is_empty());
}

#[test]
fn every_unsafe_construct_is_found() {
    let cases = [
        ("unsafe fn raw() {}", "unsafe fn raw"),
        ("unsafe impl Send for Handle {}", "unsafe impl"),
        ("unsafe trait Raw {}", "unsafe trait Raw"),
        ("extern \"C\" { fn c(); }", "extern block"),
        ("unsafe extern \"C\" { fn c(); }", "extern block"),
        ("#![allow(unsafe_code)]", "allow(unsafe_code)"),
        ("#[expect(unsafe_code)] fn f() {}", "allow(unsafe_code)"),
        (
            "#[cfg_attr(test, allow(unsafe_code))] fn f() {}",
            "allow(unsafe_code)",
        ),
        ("impl T { unsafe fn m(&self) {} }", "unsafe fn m"),
        (
            "macro_rules! m { () => { unsafe { f() } } }",
            "macro that mentions unsafe",
        ),
        (
            "fn f() { run!(unsafe { g() }); }",
            "macro that mentions unsafe",
        ),
    ];
    for (source, expected) in cases {
        let sites = sites_in("fixture", source);
        assert!(
            sites.iter().any(|site| site.what == expected),
            "{source:?} should report {expected:?}, got {sites:?}"
        );
    }
}

#[test]
fn lookalikes_are_not_findings() {
    let cases = [
        "#![forbid(unsafe_code)]",
        "#[deny(unsafe_code)] fn f() {}",
        "const WORDS: &[&str] = &[\"unsafe\", \"unsafe_code\"];",
        "// unsafe { f() }\nfn f() {}",
        "fn f() { assert!(text.contains(\"unsafe\")); }",
        "type Callback = Option<unsafe extern \"C\" fn(i32)>;",
    ];
    for source in cases {
        let sites = sites_in("fixture", source);
        assert!(sites.is_empty(), "{source:?} reported {sites:?}");
    }
}

#[test]
fn forbidden_layers_match_whole_module_names() {
    assert!(in_forbidden_layer("src/models.rs"));
    assert!(in_forbidden_layer("src/models/drive_entry.rs"));
    assert!(in_forbidden_layer("src/ui/viewer.rs"));
    assert!(in_forbidden_layer("src/command.rs"));
    assert!(!in_forbidden_layer("src/usermenu.rs"));
    assert!(!in_forbidden_layer("src/ops/command.rs"));
    assert!(!in_forbidden_layer("src/commands_extra.rs"));
}

#[test]
#[should_panic(expected = "parse broken.rs")]
fn unparsable_source_fails_loudly() {
    sites_in("broken.rs", "fn {");
}
