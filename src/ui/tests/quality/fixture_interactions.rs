//! Active command input exercises selection, suggestions and completion styles.

use crate::models::{Candidate, CandidateKind, CommandLine, Menu, Overlay};

use super::{overlay, Case};

pub(super) fn cases() -> Vec<Case> {
    let mut selected = CommandLine::from("echo Проверка 界");
    selected.line.select_all();
    let mut ghost = CommandLine::from("git");
    ghost.ghost = Some(" status --short".to_owned());
    vec![
        overlay(
            "command-selection",
            Overlay::CommandLine(selected),
            &["Проверка"],
        ),
        overlay(
            "command-suggestion",
            Overlay::CommandLine(ghost),
            &["status --short"],
        ),
        completion(),
    ]
}

fn completion() -> Case {
    let mut state = CommandLine::from("cargo ");
    state.menu = Menu::new(
        ["build", "check", "test"]
            .into_iter()
            .map(|text| Candidate {
                text: text.to_owned(),
                kind: CandidateKind::Word,
                hint: None,
            })
            .collect(),
    );
    overlay(
        "command-completion",
        Overlay::CommandLine(state),
        &["build", "check", "test"],
    )
}
