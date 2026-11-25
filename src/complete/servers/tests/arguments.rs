//! Purpose: Pin the Arguments field: the catalog's and the draft's arguments
//! only, never a transport switch.

use super::*;

#[test]
fn arguments_come_from_the_catalog_and_the_draft_only() {
    let mut draft = LspSettings::catalog().clone();
    draft.0.insert(
        "fixture".to_owned(),
        ServerSettings {
            args: vec![
                "--log=verbose".to_owned(),
                String::new(),
                "bad\u{7}".to_owned(),
            ],
            ..ServerSettings::default()
        },
    );
    let rows = offered(CompletedField::Arguments, "", &draft, &[]);
    assert_eq!(texts(&rows), ["start", "--stdio", "--log=verbose"]);
    assert_eq!(hint(&rows, "--stdio"), Some("catalog"));
    assert_eq!(hint(&rows, "--log=verbose"), Some("in use"));
    for transport in ["--socket=", "--node-ipc", "--pipe="] {
        assert!(!texts(&rows).contains(&transport), "{transport}");
    }
    let rows = offered(CompletedField::Arguments, r#"--stdio "--l"#, &draft, &[]);
    assert_eq!(texts(&rows), ["--log=verbose"]);
}
