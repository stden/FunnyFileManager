//! Purpose: The headless entry point validates arguments and messages before any application work.
use super::{arguments, options::Options, protocol::Request};
use std::ffi::OsString;

#[test]
fn ordinary_startup_is_not_intercepted_and_automation_keeps_its_arguments() {
    assert!(arguments(&[]).is_none());
    assert!(arguments(&[OsString::from("--version")]).is_none());
    let args = [OsString::from("--automation"), OsString::from("--isolated")];
    assert_eq!(arguments(&args), Some(args[1..].as_ref()));
}

#[test]
fn headless_dimensions_and_directory_are_validated_before_starting_a_session() {
    let root = tempfile::TempDir::new().unwrap();
    let args = [
        OsString::from("--root"),
        root.path().into(),
        "--width".into(),
        "80".into(),
    ];
    let options = Options::parse(&args).unwrap();
    assert_eq!(options.root, root.path().canonicalize().unwrap());
    assert_eq!(options.size.width, 80);
    assert!(Options::parse(&["--root".into()]).is_err());
    assert!(Options::parse(&[
        "--root".into(),
        root.path().into(),
        "--width".into(),
        "999".into()
    ])
    .is_err());
    assert!(Options::parse(&["--unknown".into()]).is_err());
}

#[test]
fn protocol_rejects_unknown_operations_and_accidental_extra_fields() {
    assert!(serde_json::from_str::<Request>(r#"{"operation":"unknown"}"#).is_err());
    for operation in ["state", "catalog", "cancel", "close"] {
        let value = serde_json::json!({"operation": operation, "execute": "anything"});
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    assert!(matches!(
        serde_json::from_str::<Request>(r#"{"operation":"state"}"#),
        Ok(Request::State {})
    ));
}

#[test]
fn isolated_session_serves_a_real_panel_without_a_host_terminal() {
    let root = tempfile::TempDir::new().unwrap();
    std::fs::write(root.path().join("visible.txt"), b"hello").unwrap();
    let args = ["--root".into(), root.path().into(), "--isolated".into()];
    let options = Options::parse(&args).unwrap();
    let input = std::io::Cursor::new(b"{\"operation\":\"close\"}\n");
    let mut output = Vec::new();
    assert!(super::server::serve(options, input, &mut output).is_ok());
    assert!(String::from_utf8(output).unwrap().contains("visible.txt"));
}

#[test]
fn idle_runtime_pump_reports_unchanged_after_initial_frame() {
    let root = tempfile::TempDir::new().unwrap();
    let size = ratatui::layout::Size::new(80, 25);
    let mut runtime = crate::application::headless::Runtime::new(
        root.path(),
        size,
        super::options::Profile::Isolated,
    )
    .unwrap();
    let _ = runtime.pump().unwrap();
    assert!(!runtime.pump().unwrap());
}
