//! Purpose: Pin the Extensions field: owners as hints, the field's own
//! extensions skipped, and a multibyte space ending the word.

use super::*;

#[test]
fn extensions_name_their_owners_skip_the_field_and_carry_a_dot() {
    let mut draft = LspSettings::catalog().clone();
    draft.0.insert(
        "deno".to_owned(),
        ServerSettings {
            extensions: vec!["ts".to_owned()],
            ..ServerSettings::default()
        },
    );
    let rows = offered(CompletedField::Extensions, ".t", &draft, &[]);
    assert_eq!(hint(&rows, ".ts"), Some("deno, typescript"));
    assert_eq!(hint(&rows, ".tsx"), Some("typescriptreact"));
    assert!(texts(&rows).iter().all(|text| text.starts_with('.')));

    let rows = offered(CompletedField::Extensions, "TS, .t", &draft, &[]);
    assert!(!texts(&rows).contains(&".ts"), "already in the field");
    assert!(texts(&rows).contains(&".tsx"));
}

/// The word the caret is in is being retyped, so the part of it after the
/// caret is still offered; the words after it are in the field already.
#[test]
fn the_rest_of_the_word_under_the_caret_is_still_offered() {
    let vocabulary = Vocabulary {
        catalog: LspSettings::catalog(),
        draft: LspSettings::catalog(),
        programs: &[],
    };
    let rows = candidates(
        CompletedField::Extensions,
        &line_at("", "ts rs"),
        &vocabulary,
    );
    assert!(texts(&rows).contains(&".ts"), "{:?}", texts(&rows));
    assert!(!texts(&rows).contains(&".rs"), "{:?}", texts(&rows));
}

/// A no-break space (Option+Space on macOS) or an ideographic space (CJK input
/// methods) separates extensions: the word starts after its last byte, never
/// inside it, whether the list is offered, the token spelled or one accepted.
#[test]
fn a_multibyte_space_ends_the_extension_word_after_its_last_byte() {
    for space in ['\u{a0}', '\u{3000}'] {
        let before = format!(".rs{space}p");
        assert_eq!(
            typed(CompletedField::Extensions, &line_at(&before, "")),
            ".p"
        );
        let rows = offered(
            CompletedField::Extensions,
            &before,
            LspSettings::catalog(),
            &[],
        );
        assert!(texts(&rows).contains(&".py"), "{space:?}");
        assert!(!texts(&rows).contains(&".rs"), "already in the field");

        let mut line = line_at(&before, "");
        accept(CompletedField::Extensions, &mut line, ".py");
        assert_eq!(line.text(), format!(".rs{space}.py "));

        let just_after = format!(".rs{space}");
        assert_eq!(
            typed(CompletedField::Extensions, &line_at(&just_after, "")),
            ""
        );
    }
}
