//! Purpose: Construct only complete retry requests and explain denied elevation.

use super::*;

#[test]
fn a_transfer_retry_preserves_the_dialog_and_original_selection() {
    let root = TempDir::new().unwrap();
    let sources = [root.path().join("a"), root.path().join("b")];
    let settings =
        crate::copy::CopyPrompt::for_purpose(crate::copy::Purpose::Move, "selection", root.path());
    let retry = retry_for(
        true,
        Operation::Move,
        Some(&Denial::Rights),
        &sources,
        root.path(),
        4,
        Some(&settings),
    )
    .expect("allowed complete retry");
    assert_eq!(retry.sources, sources);
    assert_eq!(retry.base, root.path());
    assert_eq!(retry.selected, 4);
    assert_eq!(retry.settings, Some(settings));
    assert_eq!(retry.operation, Operation::Move);
}

#[test]
fn incomplete_transfers_and_empty_selections_are_never_offered() {
    let root = TempDir::new().unwrap();
    let sources = [root.path().join("a")];
    for (operation, paths) in [
        (Operation::Copy, sources.as_slice()),
        (Operation::Delete, &[]),
    ] {
        assert_eq!(
            retry_for(
                true,
                operation,
                Some(&Denial::Rights),
                paths,
                root.path(),
                1,
                None
            ),
            None
        );
    }
}

#[test]
fn declining_the_administrator_prompt_explains_the_refusal() {
    let root = TempDir::new().unwrap();
    let mut app = app_in(root.path());
    report_error(&mut app, &io::Error::from_raw_os_error(CANCELLED));
    assert_eq!(
        app.status,
        Status::Error("Administrator rights were not granted".to_owned())
    );
    report_error(&mut app, &io::Error::other("helper pipe closed"));
    assert_eq!(
        app.status,
        Status::Error("Could not run as administrator: helper pipe closed".to_owned())
    );
}
