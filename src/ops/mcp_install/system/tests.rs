//! Unit tests for [`super`]: the real host has the right shape, PATHEXT is
//! parsed the way Windows reads it, and the real runner captures, reports
//! and bounds a program. The only program run is `sh`.

use std::time::Duration;

use super::*;

#[test]
fn the_system_host_reads_path_and_the_running_executable() {
    let host = system_host();
    let expected: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();
    assert_eq!(host.search_path, expected);
    assert!(host.exe_dir.is_some());
    assert!(!host.program_suffixes.is_empty());
    let named = env_vars();
    assert!(host.env.keys().all(|var| named.contains(var)));
}

#[test]
fn pathext_is_split_trimmed_and_lower_cased() {
    assert_eq!(
        parse_pathext(Some(".COM; .EXE;;.Cmd ")),
        [".com", ".exe", ".cmd"]
    );
}

#[test]
fn an_unset_or_blank_pathext_falls_back_to_the_cmd_default() {
    let default = [".com", ".exe", ".bat", ".cmd"];
    assert_eq!(parse_pathext(None), default);
    assert_eq!(parse_pathext(Some("  ")), default);
}

#[cfg(unix)]
fn shell(script: &str, timeout: Duration) -> CliCall {
    CliCall {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".to_owned(), script.to_owned()],
        cwd: env::temp_dir(),
        timeout,
    }
}

#[cfg(unix)]
#[test]
fn the_system_runner_captures_both_streams_and_the_exit_code() {
    let captured = SystemRunner
        .run(&shell(
            "echo out; echo err >&2; exit 3",
            Duration::from_secs(10),
        ))
        .unwrap();
    assert_eq!(captured.exit, Exit::Failure(Some(3)));
    assert_eq!(captured.stdout, "out\n");
    assert_eq!(captured.stderr, "err\n");
    let clean = SystemRunner
        .run(&shell("true", Duration::from_secs(10)))
        .unwrap();
    assert_eq!(clean.exit, Exit::Success);
}

#[cfg(unix)]
#[test]
fn the_system_runner_stops_a_program_at_its_timeout() {
    let error = SystemRunner
        .run(&shell("sleep 5", Duration::from_millis(100)))
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
}

#[test]
fn a_program_that_is_not_there_is_not_found() {
    let call = CliCall {
        program: env::temp_dir().join("ffm-no-such-program"),
        args: Vec::new(),
        cwd: env::temp_dir(),
        timeout: Duration::from_secs(1),
    };
    let error = SystemRunner.run(&call).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}
