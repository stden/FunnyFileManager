//! Purpose: Unit tests for user YAML `network_drives` loading, template creation, commands, and connect-all.

use super::*;
use crate::models::{MountPoint, NetworkAddress, NetworkDrive, NetworkProtocol, Panel};

fn make_drive(protocol: NetworkProtocol, path: &str, remote: &str) -> NetworkDrive {
    NetworkDrive {
        path: MountPoint::try_from(path.to_owned()).expect("mount"),
        remote: NetworkAddress::try_from(remote.to_owned()).expect("remote"),
        protocol,
    }
}

fn smb(path: &str, remote: &str) -> NetworkDrive {
    make_drive(NetworkProtocol::Smb, path, remote)
}

#[test]
fn load_from_returns_empty_when_missing_even_if_legacy_settings_json_exists() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(
        temp.path().join("settings.json"),
        r#"{"confirmations":true}"#,
    )
    .expect("json");
    let missing = temp.path().join("ffm.yml");
    assert_eq!(storage::load_from(&missing).expect("empty"), []);

    save_to(&missing, &[smb("Z:\\", "\\\\example.test\\share")]).expect("save");
    let loaded = storage::load_from(&missing).expect("loaded");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].protocol.label(), "smb");

    std::fs::write(&missing, "network_drives: [broken\n").expect("bad yaml");
    assert!(storage::load_from(&missing).is_err());
}

#[test]
fn ensure_template_creates_starter_yaml_once_without_overwriting_existing() {
    let temp = tempfile::TempDir::new().expect("temp");
    let yml = temp.path().join("nested").join("ffm.yml");
    storage::create_parent_dir(Path::new("ffm.yml")).expect("no parent");
    storage::ensure_template(&yml).expect("create");
    let created = std::fs::read_to_string(&yml).expect("read");
    assert!(created.contains("network_drives:"));
    assert_eq!(storage::load_from(&yml).expect("parses"), []);

    std::fs::write(&yml, "custom: 42\nnetwork_drives: []\n").expect("custom");
    storage::ensure_template(&yml).expect("keep existing");
    assert_eq!(
        std::fs::read_to_string(&yml).expect("read"),
        "custom: 42\nnetwork_drives: []\n"
    );
}

#[test]
fn run_with_summarizes_connected_skipped_and_failed_drives() {
    let drives = vec![
        smb("X:\\", "\\\\example.test\\one"),
        smb("Y:\\", "\\\\example.test\\two"),
        smb("Z:\\", "\\\\example.test\\three"),
    ];
    let ok = connect::run_with(&drives[..2], |d| d.path.as_str() == "X:\\", |_| Ok(()));
    assert_eq!((ok.connected, ok.skipped, ok.failed), (1, 1, 0));
    assert!(connect::format_report(ok)
        .expect("ok")
        .contains("1 connected"));

    let fail = connect::run_with(
        &drives,
        |d| d.path.as_str() == "X:\\",
        |d| {
            if d.path.as_str() == "Z:\\" {
                Err(io::Error::other("network path not found"))
            } else {
                Ok(())
            }
        },
    );
    let err = connect::format_report(fail).expect_err("failed drive errors");
    assert!(err.to_string().contains("FAIL  Z:\\"));
    assert!(connect::run(&[])
        .expect("empty run")
        .contains("0 connected"));
}

#[test]
fn mount_command_handles_all_protocols_and_windows_validation() {
    let path = if cfg!(windows) { "z:\\" } else { "/mnt/share" };
    for (protocol, remote, label) in [
        (NetworkProtocol::Smb, "//example.test/share", "smb"),
        (
            NetworkProtocol::WebDav,
            "https://example.test/dav",
            "web_dav",
        ),
        (NetworkProtocol::Nfs, "example.test:/export", "nfs"),
    ] {
        assert_eq!(protocol.label(), label);
        assert!(connect::mount_command(&make_drive(protocol, path, remote)).is_ok());
    }
    let ssh_path = if cfg!(windows) { "Z:\\" } else { "/mnt/ssh" };
    let sshfs = make_drive(NetworkProtocol::Sshfs, ssh_path, "user@example.test:/work");
    assert_eq!(NetworkProtocol::Sshfs.label(), "sshfs");
    assert_eq!(connect::mount_command(&sshfs).is_ok(), !cfg!(windows));
    #[cfg(windows)]
    assert!(connect::mount_one(&smb("/mnt/unix", "\\\\example.test\\s")).is_err());
    let mocked = if cfg!(windows) {
        smb("Z:\\", "\\\\example.test\\s")
    } else {
        sshfs
    };
    connect::mount_one_with(&mocked, |_| Ok(())).expect("runner");
}

fn shell_exit(code: i32, text: Option<&str>) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("cmd.exe");
        let script = match text {
            Some(msg) => format!("echo {msg} & exit /b {code}"),
            None => format!("exit /b {code}"),
        };
        cmd.args(["/D", "/C", &script]);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new("sh");
        let script = match text {
            Some(msg) => format!("echo {msg} >&2; exit {code}"),
            None => format!("exit {code}"),
        };
        cmd.args(["-c", &script]);
        cmd
    }
}

#[test]
fn run_command_and_is_mounted_cover_exit_codes_and_paths() {
    let temp = tempfile::TempDir::new().expect("temp");
    let existing = NetworkDrive {
        path: MountPoint::try_from(temp.path().display().to_string()).expect("dir"),
        remote: NetworkAddress::try_from("example.test:/x".to_owned()).expect("remote"),
        protocol: NetworkProtocol::Nfs,
    };
    let _ = connect::is_mounted(&existing);

    connect::run_command(&mut shell_exit(0, None)).expect("exit 0");
    assert!(connect::run_command(&mut shell_exit(1, Some("detail-msg")))
        .unwrap_err()
        .to_string()
        .contains("detail-msg"));
    assert!(connect::run_command(&mut shell_exit(1, None))
        .unwrap_err()
        .to_string()
        .contains("failed"));
}

#[test]
fn ops_queue_edit_list_and_execute_work_against_temp_config() {
    use crate::test_support::{EnvVarGuard, CONFIG_HOME};

    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    let yml = temp.path().join("ffm.yml");
    assert_eq!(
        config_path_from(None).err().map(|e| e.kind()),
        Some(io::ErrorKind::NotFound)
    );
    list_from(&mut app, &yml).expect("empty list");
    assert!(app.status.to_string().contains("No network drives"));
    assert!(queue_connect_all_from(&mut app, &yml)
        .expect("empty")
        .to_string()
        .contains("No network drives"));

    save_to(&yml, &[smb("Z:\\", "\\\\example.test\\share")]).expect("save");
    let status = queue_connect_all_from(&mut app, &yml).expect("queue");
    assert!(status.to_string().contains("Connecting 1"));
    assert!(app.maintenance_request.is_some());
    list_from(&mut app, &yml).expect("viewer");
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
    edit_config_at(&mut app, &yml).expect("edit");
    assert!(matches!(app.overlay, Overlay::Editor(_)));

    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    execute(&mut app, NetDrivesCommand::Edit).expect("cmd edit");
    execute(&mut app, NetDrivesCommand::List).expect("cmd list");
    execute(&mut app, NetDrivesCommand::Connect).expect("cmd connect");
}
