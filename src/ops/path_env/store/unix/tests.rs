use super::*;
use tempfile::TempDir;

#[test]
fn profile_save_preserves_other_settings_and_refuses_external_changes() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("profile");
    fs::write(&path, "export EDITOR=vi\n").expect("fixture");
    let snapshot = read_at(&path, Format::Profile, "/bin").expect("read");
    write_expanded_at(&path, Format::Profile, &snapshot, "/bin:/opt/a'b'").expect("write");
    let after = read_at(&path, Format::Profile, "").expect("read");
    assert_eq!(after.value, "/bin:/opt/a'b'");
    assert!(after.revision.starts_with("export EDITOR=vi"));
    assert!(path.with_extension("ffm-path.bak").exists());
    assert!(write_at(&path, Format::Profile, &snapshot, "/bin").is_err());
}

#[test]
fn shell_profiles_follow_login_precedence_within_the_selected_home() {
    let temp = TempDir::new().expect("home");
    let selected = |shell| user_profile(temp.path(), shell);
    assert_eq!(selected("/bin/zsh"), temp.path().join(".zprofile"));
    assert_eq!(selected("/bin/sh"), temp.path().join(".profile"));
    fs::write(temp.path().join(".bash_login"), "").expect("login");
    assert_eq!(selected("/bin/bash"), temp.path().join(".bash_login"));
    fs::write(temp.path().join(".bash_profile"), "").expect("profile");
    assert_eq!(selected("/bin/bash"), temp.path().join(".bash_profile"));
}

#[test]
fn system_formats_keep_other_environment_variables() {
    assert_eq!(
        replace("LANG=en\nPATH=\"/bin\"\n", Format::Environment, "/usr/bin").expect("format"),
        "LANG=en\nPATH=\"/usr/bin\"\n"
    );
    assert_eq!(
        replace("", Format::Paths, "/bin:/usr/bin").expect("format"),
        "/bin\n/usr/bin\n"
    );
    assert!(profile_block(START).is_err());
}

#[test]
fn system_files_round_trip_without_losing_other_settings() {
    let temp = TempDir::new().expect("tempdir");
    for (format, initial) in [
        (Format::Paths, "# system paths\n/bin\n\n/usr/bin\n"),
        (Format::Environment, "LANG=en\nPATH=/bin:/usr/bin\n"),
    ] {
        let path = temp.path().join("system");
        fs::write(&path, initial).expect("fixture");
        let before = read_at(&path, format, "").expect("read");
        assert_eq!(before.value, "/bin:/usr/bin");
        write_at(&path, format, &before, "/opt/bin").expect("save");
        assert_eq!(read_at(&path, format, "").expect("read").value, "/opt/bin");
        assert_eq!(
            contents(&path.with_extension("ffm-path.bak")).expect("backup"),
            initial
        );
    }
}

#[test]
fn profiles_are_created_and_updated_without_duplicate_blocks() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("new-profile");
    let initial = read_at(&path, Format::Profile, "/bin").expect("new profile");
    write_at(&path, Format::Profile, &initial, "/usr/bin").expect("create");
    let current = read_at(&path, Format::Profile, "").expect("profile");
    write_at(&path, Format::Profile, &current, "/opt/bin").expect("replace");
    let updated = read_at(&path, Format::Profile, "").expect("updated");
    assert_eq!(updated.value, "/opt/bin");
    assert_eq!(updated.revision.matches(START).count(), 1);
    assert!(read_at(temp.path(), Format::Profile, "").is_err());
    assert!(replace("", Format::Environment, "/path/with\"quote").is_err());
}

#[test]
fn writes_refuse_profile_links_and_replace_backup_links_without_following_them() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("profile");
    let other = temp.path().join("other-file");
    fs::write(&other, "keep me").expect("other file");
    std::os::unix::fs::symlink(&other, &path).expect("profile link");
    let before = read_at(&path, Format::Profile, "/bin").expect("snapshot");
    assert!(write_at(&path, Format::Profile, &before, "/usr/bin").is_err());
    fs::remove_file(&path).expect("remove fixture link");
    fs::write(&path, "export EDITOR=vi\n").expect("profile");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("mode");
    let backup = path.with_extension("ffm-path.bak");
    std::os::unix::fs::symlink(&other, &backup).expect("backup link");
    let before = read_at(&path, Format::Profile, "/bin").expect("snapshot");
    write_at(&path, Format::Profile, &before, "/usr/bin").expect("save");
    assert_eq!(contents(&other).expect("untouched other file"), "keep me");
    assert!(!fs::symlink_metadata(&backup)
        .expect("backup")
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::metadata(&backup).expect("mode").permissions().mode() & 0o777,
        0o600
    );
}

// ZZPROBE136-BEGIN
#[test]
fn zz_probe136_write_at_propagates_non_notfound_read_error() {
    let temp = TempDir::new().expect("tempdir");
    let snapshot = read_at(&temp.path().join("profile"), Format::Profile, "/bin").expect("read");
    let error = write_at(temp.path(), Format::Profile, &snapshot, "/bin").expect_err("is a dir");
    assert_eq!(
        error.to_string(),
        format!("{} is not a regular file", temp.path().display())
    );
}
// ZZPROBE136-END

/// Saving the PATH of all users as a guest said `Permission denied (os
/// error 13) at path "/etc/.tmp9N39xw"`: the temp file, and not a word
/// about what the file needs.
#[test]
fn a_refused_save_names_the_path_file_and_the_rights_it_needs() {
    let temp_file = || {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Permission denied (os error 13) at path \"/etc/.tmp9N39xw\"",
        )
    };
    let system = refused(Scope::System, Path::new("/etc/environment"), temp_file());
    assert_eq!(
        system.to_string(),
        "/etc/environment: permission denied; the PATH of all users needs administrator rights"
    );
    assert_eq!(system.kind(), io::ErrorKind::PermissionDenied);
    let user = refused(Scope::User, Path::new("/home/bob/.profile"), temp_file());
    assert_eq!(user.to_string(), "/home/bob/.profile: permission denied");
    let other = refused(
        Scope::System,
        Path::new("/etc/environment"),
        io::Error::other(super::super::CHANGED_OUTSIDE),
    );
    assert_eq!(other.to_string(), super::super::CHANGED_OUTSIDE);
}

/// A FIFO where the profile goes: the PATH editor refuses it, naming it,
/// instead of waiting for a writer.
#[test]
fn a_profile_that_is_not_a_regular_file_is_refused_without_waiting() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join(".profile");
    crate::test_support::make_fifo(&path).expect("fifo");
    let shown = path.display().to_string();
    let outcome =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            read_at(&path, Format::Profile, "/bin").map(|snapshot| snapshot.value)
        })
        .expect("the profile is not waited on");
    assert_eq!(
        outcome.map_err(|error| error.to_string()),
        Err(format!("{shown} is not a regular file"))
    );
}

/// Under an empty `HOME=` the profile was `.profile` in the launch directory,
/// and a save wrote a PATH block there. Without an absolute home the user's
/// PATH has no profile, and the editor says why.
#[test]
fn without_an_absolute_home_the_user_profile_has_no_place() {
    for home in ["", "relative"] {
        let _home = crate::test_support::EnvVarGuard::set("HOME", Path::new(home));
        let Err(error) = location(Scope::User) else {
            panic!("HOME={home:?} is no home");
        };
        assert_eq!(error.to_string(), NO_HOME);
    }
}
