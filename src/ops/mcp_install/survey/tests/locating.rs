//! Purpose: Finding programs on PATH and the server beside `ffm`.

use super::*;

#[test]
fn find_program_tries_each_suffix_in_each_folder_and_the_first_folder_wins() {
    let machine = machine();
    let second = machine.home().join("second-bin");
    fs::create_dir(&second).unwrap();
    crate::test_support::write_executable(&second.join("claude.cmd"), b"x").unwrap();
    let first = machine.install("claude").unwrap();
    let mut host = machine.host();
    host.search_path.push(second.clone());
    host.program_suffixes = vec![String::new(), ".cmd".to_owned()];
    assert_eq!(find_program(&host, "claude"), Some(first.clone()));
    fs::remove_file(&first).unwrap();
    assert_eq!(
        find_program(&host, "claude"),
        Some(second.join("claude.cmd"))
    );
}

#[test]
fn find_program_skips_relative_folders_and_an_empty_path() {
    let machine = machine();
    machine.install("claude").unwrap();
    let mut host = machine.host();
    host.search_path = vec![PathBuf::from("relative/bin")];
    assert_eq!(find_program(&host, "claude"), None);
    host.search_path.clear();
    assert_eq!(find_program(&host, "claude"), None);
}

#[cfg(unix)]
#[test]
fn a_file_without_an_execute_bit_is_not_a_program() {
    let machine = machine();
    fs::write(machine.bin().join("codex"), b"not runnable").unwrap();
    assert_eq!(find_program(&machine.host(), "codex"), None);
    fs::create_dir(machine.bin().join("junie")).unwrap();
    assert_eq!(find_program(&machine.host(), "junie"), None);
}

#[test]
fn the_server_next_to_ffm_beats_the_one_on_path() {
    let machine = machine();
    let on_path = machine.install("ffm-mcp").unwrap();
    assert_eq!(
        locate_server(&machine.host()),
        ServerState::Found(ServerBinary::new(on_path).unwrap())
    );
    let beside = machine.install_server().unwrap();
    assert_eq!(
        locate_server(&machine.host()),
        ServerState::Found(ServerBinary::new(beside).unwrap())
    );
}

#[test]
fn no_server_anywhere_is_missing() {
    let machine = machine();
    assert_eq!(locate_server(&machine.host()), ServerState::Missing);
    assert_eq!(locate_server(&Host::default()), ServerState::Missing);
}

#[cfg(unix)]
#[test]
fn a_server_whose_path_is_not_unicode_is_named_but_not_found() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let machine = machine();
    let odd = machine
        .home()
        .join(OsString::from_vec(b"bin-\xff".to_vec()));
    if !crate::test_support::filesystem::non_unicode_name(fs::create_dir(&odd)).unwrap() {
        return;
    }
    crate::test_support::write_executable(&odd.join("ffm-mcp"), b"x").unwrap();
    let mut host = machine.host();
    host.exe_dir = Some(odd.clone());
    assert_eq!(
        locate_server(&host),
        ServerState::NotUnicode(odd.join("ffm-mcp"))
    );
}
