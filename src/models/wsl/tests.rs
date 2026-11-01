//! Domain validation, bounded selection and command outbox semantics.

use super::*;

fn names(values: &[&str]) -> Vec<DistributionName> {
    values
        .iter()
        .map(|value| DistributionName::new(value).expect("valid name"))
        .collect()
}

#[test]
fn distribution_names_form_one_unc_share_component() {
    for name in ["Ubuntu", "Debian-12", "My Linux", "Linux-日本語"] {
        let distribution = DistributionName::new(name).expect("valid distribution");
        assert_eq!(distribution.as_str(), name);
        assert_eq!(
            distribution.root_path(),
            PathBuf::from(format!("\\\\wsl.localhost\\{name}\\"))
        );
    }
}

#[test]
fn distribution_names_reject_invalid_share_components() {
    for invalid in [
        "",
        " ",
        " Ubuntu",
        "Ubuntu ",
        ".",
        "..",
        "Ubuntu.",
        "../Ubuntu",
        "Ubuntu\\root",
        "Ubuntu:root",
        "Ubuntu\n",
        "Ubuntu\0",
        "a*b",
        "a?b",
        "a\"b",
        "a<b",
        "a>b",
        "a|b",
    ] {
        assert!(
            DistributionName::new(invalid).is_none(),
            "accepted {invalid:?}"
        );
    }
}

#[test]
fn empty_selection_stays_empty_through_every_navigation() {
    let mut dialog = WslDialog::new(Vec::new());
    dialog.move_up();
    dialog.move_down();
    dialog.first();
    dialog.last();
    assert_eq!(dialog.distributions().len(), 0);
    assert_eq!(dialog.selected(), None);
    assert_eq!(dialog.selected_distribution(), None);
    assert_eq!(dialog.activity, WslActivity::Ready);
}

#[test]
fn navigation_clamps_and_refresh_preserves_selection_by_name() {
    let mut dialog = WslDialog::new(names(&["Ubuntu", "Debian", "Alpine"]));
    dialog.move_up();
    assert_eq!(dialog.selected(), Some(0));
    dialog.last();
    dialog.move_down();
    assert_eq!(dialog.selected(), Some(2));
    dialog.move_up();
    assert_eq!(
        dialog.selected_distribution().map(DistributionName::as_str),
        Some("Debian")
    );
    dialog.replace_distributions(names(&["Debian", "Ubuntu"]));
    assert_eq!(dialog.selected(), Some(0));
    assert_eq!(
        dialog.selected_distribution().map(DistributionName::as_str),
        Some("Debian")
    );
    dialog.replace_distributions(names(&["Alpine", "Ubuntu"]));
    assert_eq!(dialog.selected(), Some(0));
    dialog.last();
    dialog.first();
    assert_eq!(dialog.selected(), Some(0));
    dialog.replace_distributions(Vec::new());
    assert_eq!(dialog.selected(), None);
    dialog.replace_distributions(names(&["Ubuntu"]));
    assert_eq!(dialog.selected(), Some(0));
}

#[test]
fn taking_requests_drains_them_and_cloning_does_not_replay_effects() {
    let mut requests = WslRequests::default();
    let first = requests.enqueue(WslOperation::Refresh);
    let second = requests.enqueue(WslOperation::Update);
    assert_eq!(first, WslRequestId::new(1));
    assert_ne!(first, second);
    assert_eq!(requests.pending().len(), 2);
    let mut cloned = requests.clone();
    assert_eq!(cloned.pending().len(), 0);
    assert_ne!(cloned.enqueue(WslOperation::Refresh), second);
    let drained = requests.take();
    assert_eq!(drained[0].id, first);
    assert_eq!(drained[0].operation, WslOperation::Refresh);
    assert_eq!(drained[1].id, second);
    assert_eq!(drained[1].operation, WslOperation::Update);
    assert_eq!(requests.pending().len(), 0);
    assert_eq!(requests.take().len(), 0);
}
