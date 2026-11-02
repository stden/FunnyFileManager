//! Purpose: State never depends on a higher layer, apart from the reported inversion.

use super::*;

pub fn models_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("models");
    let mut violations = forbidden_dependencies(sources, checked, &MODELS_FORBIDDEN);
    violations.retain(|violation| !MODELS_KNOWN_INVERSIONS.contains(&violation.path.as_str()));
    messages(&violations)
}

#[test]
fn models_rule_reports_upward_dependencies() {
    let sources = sources(&[
        (
            "src/models/panel.rs",
            "use super::super::ui::panel_page_size;\n",
        ),
        (
            "src/models/editor.rs",
            "use crate::models::editor::Cursor;\n",
        ),
    ]);
    assert_eq!(
        models_violations(&sources),
        ["src/models/panel.rs:1: `crate::ui::panel_page_size` (forbidden: `crate::ui`)"]
    );
}

/// The models define the process ports and the process table fills them, so
/// the models never name it; their one existing `platform` use stays legal.
#[test]
fn models_rule_rejects_the_process_table_but_not_platform() {
    let sources = sources(&[
        ("src/models/tasks.rs", "use crate::process_table::ProcFs;\n"),
        (
            "src/models/drive_entry.rs",
            "use crate::platform::DriveKind;\n",
        ),
    ]);
    assert_eq!(
        models_violations(&sources),
        ["src/models/tasks.rs:1: `crate::process_table::ProcFs` (forbidden: `crate::process_table`)"]
    );
}

#[test]
fn models_rule_tolerates_only_the_reported_inversion() {
    let sources = sources(&[
        ("src/lib.rs", "mod application;\npub use application::{run_app, CLIPBOARD_CLEAR};\n"),
        (
            "src/models/clipboard.rs",
            "fn delay() -> Duration {\n    crate::CLIPBOARD_CLEAR\n}\n\nfn restart() {\n    crate::run_app();\n}\n",
        ),
    ]);
    assert_eq!(
        models_violations(&sources),
        ["src/models/clipboard.rs:6: `crate::application::run_app` (forbidden: `crate::application`)"]
    );
}
