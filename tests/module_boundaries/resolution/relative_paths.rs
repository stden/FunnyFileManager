//! Purpose: Relative module paths: `super` chains, inline modules, `self` and crate-root
//! re-exports.

use super::Case;

pub(super) const GRANDPARENT_MODULE: Case = Case {
    name: "grandparent_module",
    file: "src/ui/panel.rs",
    source: "use super::super::ops;",
    expected: &["crate::ops"],
};

pub(super) const GREAT_GRANDPARENT_GROUP: Case = Case {
    name: "great_grandparent_group",
    file: "src/popup/nested/commands.rs",
    source: "use super::super::super::{ops, handlers};",
    expected: &["crate::ops", "crate::handlers"],
};

pub(super) const INLINE_MODULE_PARENT: Case = Case {
    name: "inline_module_parent",
    file: "src/ui.rs",
    source: "mod widgets {
    pub fn refresh(app: &mut App) {
        super::super::ops::refresh(app);
    }
}",
    expected: &["crate::ops::refresh"],
};

pub(super) const SELF_CHILD_LOOKALIKE: Case = Case {
    name: "self_child_lookalike",
    file: "src/ui.rs",
    source: r#"mod fs {
    pub fn label() -> &'static str {
        "fs"
    }
}

fn title() -> &'static str {
    self::fs::label()
}

fn heading() -> &'static str {
    fs::label()
}"#,
    expected: &[],
};

pub(super) const CRATE_ROOT_REEXPORT: Case = Case {
    name: "crate_root_reexport",
    file: "src/ui/status.rs",
    source: "fn restart() {
    crate::run_app();
}",
    expected: &["crate::application::run_app"],
};
