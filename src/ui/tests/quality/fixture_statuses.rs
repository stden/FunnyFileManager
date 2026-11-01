//! Real status states and a deterministic, isolated encrypted fixture vault.

use crate::models::{
    DistributionName, Overlay, WslActivity, WslDialog, WslOperation, WslRequestId,
};

use super::{overlay, Case};

pub(super) fn cases() -> Vec<Case> {
    let cases = [
        wsl(
            "wsl-busy",
            WslActivity::Busy {
                id: WslRequestId::new(1),
                operation: WslOperation::Update,
            },
            "Updating WSL",
        ),
        wsl(
            "wsl-failed",
            WslActivity::Failed("Fixture failure".to_owned()),
            "Error:",
        ),
    ];
    cases.into_iter().chain(vaults()).collect()
}

fn wsl(name: &'static str, activity: WslActivity, required: &'static str) -> Case {
    let mut dialog = WslDialog::new(vec![DistributionName::new("Debian").unwrap()]);
    dialog.activity = activity;
    overlay(name, Overlay::Wsl(Box::new(dialog)), &["WSL", required])
}

#[cfg(feature = "keepass")]
fn vaults() -> Vec<Case> {
    [
        ("vault-tree", VaultView::Tree),
        ("vault-list", VaultView::List),
        ("vault-filter", VaultView::Filter),
    ]
    .into_iter()
    .map(|(name, view)| vault(name, view))
    .collect()
}

#[cfg(not(feature = "keepass"))]
fn vaults() -> Vec<Case> {
    Vec::new()
}

#[cfg(feature = "keepass")]
enum VaultView {
    Tree,
    List,
    Filter,
}

#[cfg(feature = "keepass")]
fn vault(name: &'static str, view: VaultView) -> Case {
    use crate::kdbx::{
        tests_support::{type_password, vault_file, VAULT_PASSWORD},
        KdbxSession,
    };

    let (temp, path) = vault_file(ENTRIES, VAULT_PASSWORD, None);
    let mut session = KdbxSession::new(path);
    let now = std::time::Instant::now();
    type_password(&mut session, VAULT_PASSWORD, now);
    assert!(
        session.unlock(now),
        "the isolated fixture vault must unlock"
    );
    session.path = "/ffm-quality/vault.kdbx".into();
    let vault = session.vault_mut().unwrap();
    if !matches!(view, VaultView::Tree) {
        vault.toggle_structure();
    }
    if matches!(view, VaultView::Filter) {
        vault.set_filter("Example", now);
    }
    let mut case = overlay(
        name,
        Overlay::Kdbx(crate::models::KdbxOverlay(session)),
        &["Filter:"],
    );
    case.resources.push(temp);
    case
}

#[cfg(feature = "keepass")]
const ENTRIES: &[crate::kdbx::tests_support::Fixture] = &[crate::kdbx::tests_support::Fixture {
    group: &["Demo"],
    title: "Example",
    username: "reader",
    url: "https://example.test",
    password: "visual-fixture-password",
}];
