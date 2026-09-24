//! Purpose: Typed bursts and the language-servers dialog in the real binary.

use super::*;

/// A burst lands in one read, so crossterm parses all of it at once and keeps
/// the rest in its own queue, where no wait on the terminal can see it. Each
/// queued key used to cost the whole 100 ms idle poll: this line took about
/// eight seconds to arrive, one character every tenth of a second.
#[test]
fn a_burst_of_typed_keys_is_handled_without_waiting_between_keys() {
    let mut ffm = Ffm::start();
    let name = "abcdefghijklmnopqrstuvwxyz0123456789".repeat(2);

    ffm.type_keys(format!(":mkdir {name}\r").as_bytes());
    let folder = ffm.work().join(&name);
    let took = ffm.wait_for(PATIENCE, |_| folder.is_dir());

    let took = took.expect("the typed command never ran");
    assert!(
        took < Duration::from_secs(3),
        "80 queued keys took {took:?}; they must not wait for each other"
    );
}

/// The Language servers dialog says what this machine would start, so the
/// program has to hand it the `PATH` it runs with: a server whose command is
/// there is found. Every other test gives the dialog a fake machine.
#[test]
fn the_language_servers_dialog_finds_a_server_on_the_path_ffm_runs_with() {
    use std::os::unix::fs::PermissionsExt as _;

    let sandbox = TempDir::new().expect("sandbox");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&bin).expect("bin");
    let server = bin.join("aaa-fixture-server");
    fs::write(&server, "#!/bin/sh\n").expect("server");
    fs::set_permissions(&server, fs::Permissions::from_mode(0o755)).expect("chmod");
    let config = sandbox.path().join("home/.config/ffm");
    fs::create_dir_all(&config).expect("config");
    fs::write(
        config.join("settings.yaml"),
        "lsp:\n  aaa: {command: aaa-fixture-server, extensions: [aaafixture], install: null}\n",
    )
    .expect("settings");
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&inherited)))
        .expect("PATH");
    let mut ffm = Ffm::start_in(sandbox, &path);

    ffm.type_keys(b":lsp\r");
    // The status is checked before the dialog is drawn, so the first frame
    // that shows aaa's status shows the answer.
    let found = b"found configured executable";
    let shown = ffm.wait_for(PATIENCE, |ffm| {
        contains(&ffm.screen, found) || contains(&ffm.screen, b"not installed")
    });

    assert!(shown.is_some(), "the dialog never showed aaa's status");
    assert!(
        contains(&ffm.screen, found),
        "aaa is not found on PATH:\n{}",
        String::from_utf8_lossy(&ffm.screen)
    );
}
