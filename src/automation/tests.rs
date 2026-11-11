//! Purpose: The headless entry point validates arguments and messages before any application work.
use super::{arguments, options::Options, protocol::Request};
use std::ffi::OsString;

mod closed_stream;

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

#[test]
fn isolated_server_recovers_from_invalid_frames_and_accepts_cancel_before_close() {
    let root = tempfile::TempDir::new().unwrap();
    let options =
        Options::parse(&["--root".into(), root.path().into(), "--isolated".into()]).unwrap();
    let input = std::io::Cursor::new(b"invalid\n{\"operation\":\"state\"}\n{\"operation\":\"cancel\"}\n{\"operation\":\"close\"}\n");
    let mut output = Vec::new();
    super::server::serve(options, input, &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let replies: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 5);
    assert_eq!(replies[0]["result"]["protocol_version"], 1);
    assert_eq!(replies[1]["error"]["code"], "invalid_input");
    assert_eq!(replies[2]["result"]["state"]["running"], true);
    assert_eq!(replies[3]["ok"], true);
    assert_eq!(replies[4]["result"]["state"]["running"], false);
}

#[test]
fn options_validate_height_width_lower_bound_missing_root_and_regular_file() {
    let root = tempfile::TempDir::new().unwrap();
    let file = root.path().join("not-a-dir.txt");
    std::fs::write(&file, b"x").unwrap();
    let ok = Options::parse(&[
        "--root".into(),
        root.path().into(),
        "--width".into(),
        "20".into(),
        "--height".into(),
        "200".into(),
        "--isolated".into(),
    ])
    .unwrap();
    assert_eq!((ok.size.width, ok.size.height), (20, 200));
    assert!(matches!(ok.profile, super::options::Profile::Isolated));
    for (flag, val) in [
        ("--width", "19"),
        ("--height", "7"),
        ("--height", "201"),
        ("--height", "nope"),
    ] {
        assert!(
            Options::parse(&["--root".into(), root.path().into(), flag.into(), val.into()])
                .is_err()
        );
    }
    assert!(Options::parse(&["--isolated".into()]).is_err());
    assert!(Options::parse(&["--root".into(), file.into()]).is_err());
}

struct BrokenPipeReader;

impl std::io::Read for BrokenPipeReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe closed",
        ))
    }
}

#[test]
fn server_stops_cleanly_on_eof() {
    let root = tempfile::TempDir::new().unwrap();
    let options =
        Options::parse(&["--root".into(), root.path().into(), "--isolated".into()]).unwrap();
    let mut eof_out = Vec::new();
    super::server::serve(options, std::io::Cursor::new(b""), &mut eof_out).unwrap();
    assert_eq!(String::from_utf8(eof_out).unwrap().lines().count(), 1);
}

#[test]
fn server_terminates_after_fatal_read_error() {
    let root = tempfile::TempDir::new().unwrap();
    let options =
        Options::parse(&["--root".into(), root.path().into(), "--isolated".into()]).unwrap();
    let mut err_out = Vec::new();
    super::server::serve(
        options,
        std::io::BufReader::new(BrokenPipeReader),
        &mut err_out,
    )
    .unwrap();
    let lines: Vec<serde_json::Value> = String::from_utf8(err_out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["ok"].as_bool(), Some(true));
    assert_eq!(lines[1]["ok"].as_bool(), Some(false));
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "write closed",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn server_propagates_initial_greeting_write_failure() {
    let root = tempfile::TempDir::new().unwrap();
    let options =
        Options::parse(&["--root".into(), root.path().into(), "--isolated".into()]).unwrap();
    let err = super::server::serve(options, std::io::Cursor::new(b""), &mut FailingWriter)
        .expect_err("greeting write must fail");
    assert!(err.to_string().contains("write closed"));
}
