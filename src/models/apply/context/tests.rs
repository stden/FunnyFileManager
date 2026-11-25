//! Purpose: Unit tests for `PanelContext::folder` separator handling.

use std::path::MAIN_SEPARATOR;

use super::super::symbols::{NameForm, PathKind, Symbol};
use super::{Names, PanelContext};

#[test]
fn folder_appends_separator_only_when_not_already_terminated() {
    let mut panel = PanelContext::default();
    panel.paths.logical = Names {
        long: "/".to_owned(),
        short: "C:\\".to_owned(),
    };
    panel.paths.resolved = Names {
        long: "/plain".to_owned(),
        short: "PLAIN".to_owned(),
    };

    assert_eq!(
        panel.expand(Symbol::Folder(NameForm::Long, PathKind::Logical)),
        "/"
    );
    assert_eq!(
        panel.expand(Symbol::Folder(NameForm::Short, PathKind::Logical)),
        "C:\\"
    );
    assert_eq!(
        panel.expand(Symbol::Folder(NameForm::Long, PathKind::Resolved)),
        format!("/plain{MAIN_SEPARATOR}")
    );
}
