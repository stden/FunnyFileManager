use super::*;

#[test]
fn file_association_rejects_nul_paths_before_calling_the_shell() {
    let error = open_associated(Path::new("invalid\0document.txt")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn shell_execute_info_suppresses_blocking_shell_error_dialogs() {
    let verb = [0u16];
    let file = [0u16];
    let args = [0u16];
    let info = shell_execute_info(&verb, &file, &args);
    assert_ne!(info.fMask & SEE_MASK_FLAG_NO_UI, 0);
    assert_ne!(info.fMask & SEE_MASK_NOCLOSEPROCESS, 0);
    assert_ne!(info.fMask & SEE_MASK_NOASYNC, 0);
    assert_eq!(info.nShow, SW_HIDE);
}

#[test]
fn elevation_token_query_and_nul_argument_guards_run_without_uac() {
    let first = is_elevated();
    assert_eq!(first, is_elevated());
    assert_eq!(
        launch_as_administrator(Path::new("bad\0.exe"), OsStr::new(""))
            .map(|_| ())
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        launch_as_administrator(Path::new("cmd.exe"), OsStr::new("bad\0arg"))
            .map(|_| ())
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn elevated_process_tracks_child_pid_and_exit_state() {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut exited = Command::new("cmd.exe")
        .args(["/D", "/C", "exit /b 0"])
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    let exited_pid = exited.id();
    exited.wait().unwrap();
    let wrapped_exited = ElevatedProcess::from_child(exited);
    assert_eq!(wrapped_exited.id(), exited_pid);
    assert!(wrapped_exited.has_exited());

    let running = Command::new("ping.exe")
        .args(["-n", "2", "127.0.0.1"])
        .stdout(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    let running_pid = running.id();
    let wrapped_running = ElevatedProcess::from_child(running);
    assert_eq!(wrapped_running.id(), running_pid);
    assert!(!wrapped_running.has_exited());
    for _ in 0..100 {
        if wrapped_running.has_exited() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(wrapped_running.has_exited());
}
