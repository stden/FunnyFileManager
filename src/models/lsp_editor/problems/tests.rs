//! Purpose: Keep loaded extension conflicts repairable while rejecting new claims.

use super::*;

fn conflicting_settings() -> LspSettings {
    LspSettings(
        [("alpha", "Custom"), ("beta", "CUSTOM"), ("gamma", "custom")]
            .into_iter()
            .map(|(language, extension)| {
                (
                    language.to_owned(),
                    ServerSettings {
                        extensions: vec![extension.to_owned()],
                        ..ServerSettings::default()
                    },
                )
            })
            .collect(),
    )
}

fn duplicate(first: &str, second: &str, impact: Impact) -> (Problem, Impact) {
    (
        Problem::DuplicateExtension {
            extension: "custom".to_owned(),
            first: LanguageId::from_key(first),
            second: LanguageId::from_key(second),
        },
        impact,
    )
}

#[test]
fn disabling_the_first_claimant_keeps_the_remaining_loaded_conflict_a_warning() {
    let baseline = conflicting_settings();
    let mut draft = baseline.clone();
    draft.0.get_mut("alpha").unwrap().enabled = Enablement::Disabled;
    assert_eq!(
        problems(&draft, &baseline),
        [duplicate("beta", "gamma", Impact::Warning)]
    );
}

#[test]
fn deleting_the_first_claimant_keeps_the_remaining_loaded_conflict_a_warning() {
    let baseline = conflicting_settings();
    let mut draft = baseline.clone();
    draft.0.remove("alpha");
    assert_eq!(
        problems(&draft, &baseline),
        [duplicate("beta", "gamma", Impact::Warning)]
    );
}

#[test]
fn changing_the_first_claim_keeps_the_remaining_loaded_conflict_a_warning() {
    let baseline = conflicting_settings();
    let mut draft = baseline.clone();
    draft.0.get_mut("alpha").unwrap().extensions = vec!["other".to_owned()];
    assert_eq!(
        problems(&draft, &baseline),
        [duplicate("beta", "gamma", Impact::Warning)]
    );
}

#[test]
fn enabling_a_previously_disabled_claimant_introduces_blocking_conflicts() {
    let mut baseline = conflicting_settings();
    baseline.0.get_mut("alpha").unwrap().enabled = Enablement::Disabled;
    let draft = conflicting_settings();
    assert_eq!(
        problems(&draft, &baseline),
        [
            duplicate("alpha", "beta", Impact::Blocking),
            duplicate("alpha", "gamma", Impact::Blocking),
        ]
    );
}

#[test]
fn adding_a_claimant_to_an_existing_conflict_still_blocks() {
    let mut baseline = conflicting_settings();
    baseline.0.remove("gamma");
    let draft = conflicting_settings();
    assert_eq!(
        problems(&draft, &baseline),
        [
            duplicate("alpha", "beta", Impact::Warning),
            duplicate("alpha", "gamma", Impact::Blocking),
        ]
    );
}

#[test]
fn changing_an_enabled_servers_extensions_can_introduce_a_blocking_conflict() {
    let mut baseline = conflicting_settings();
    baseline.0.get_mut("gamma").unwrap().extensions = vec!["other".to_owned()];
    let draft = conflicting_settings();
    assert_eq!(
        problems(&draft, &baseline),
        [
            duplicate("alpha", "beta", Impact::Warning),
            duplicate("alpha", "gamma", Impact::Blocking),
        ]
    );
}

#[test]
fn each_problem_identifies_the_first_affected_language() {
    let language = LanguageId::from_key("fixture");
    let issues = [
        Problem::DuplicateExtension {
            extension: "rs".to_owned(),
            first: language.clone(),
            second: LanguageId::from_key("other"),
        },
        Problem::InvalidExtension {
            language: language.clone(),
            extension: "bad extension".to_owned(),
        },
        Problem::EmptyCommand {
            language: language.clone(),
        },
        Problem::NoExtensions {
            language: language.clone(),
        },
        Problem::ControlCharacter {
            language: language.clone(),
            field: OwnedField::Command,
        },
    ];
    for issue in issues {
        assert_eq!(issue.language(), &language);
        assert!(issue.involves(&language));
        assert!(!issue.involves(&LanguageId::from_key("unrelated")));
    }
}
