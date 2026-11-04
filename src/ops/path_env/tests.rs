use super::*;
use tempfile::TempDir;

#[test]
fn automatic_validation_distinguishes_missing_files_and_directories() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("a file");
    std::fs::write(&file, b"x").expect("file");
    assert_eq!(
        validate(&temp.path().to_string_lossy()),
        Validation::Directory
    );
    assert_eq!(validate(&file.to_string_lossy()), Validation::NotDirectory);
    assert_eq!(
        validate(&temp.path().join("missing").to_string_lossy()),
        Validation::Missing
    );
    assert_eq!(validate("relative/path"), Validation::Relative);
    assert_eq!(validate(""), Validation::Empty);
}

#[test]
fn saving_is_scoped_validated_and_does_not_touch_the_host_environment() {
    let temp = TempDir::new().expect("tempdir");
    let value = temp.path().to_string_lossy().into_owned();
    let snapshot = Snapshot {
        value: value.clone(),
        revision: "original".to_owned(),
        location: "fixture".to_owned(),
    };
    let mut dialog = PathEditor::new(Scope::System, snapshot);
    let mut saved = None;
    dialog.mode = Mode::ConfirmSave;
    save_with(&mut dialog, |scope, snapshot, value| {
        saved = Some((scope, value.to_owned()));
        Ok(Snapshot {
            value: value.to_owned(),
            revision: "saved".to_owned(),
            location: snapshot.location.clone(),
        })
    })
    .expect("save");
    assert_eq!(saved, Some((Scope::System, value)));
    assert!(matches!(dialog.mode, Mode::Browse));
    assert!(!dialog.dirty());
    assert!(dialog.message.contains("PATH saved"));
    dialog.entries[0].text = temp.path().join("missing").to_string_lossy().into_owned();
    assert!(save_with(&mut dialog, |_, _, _| panic!(
        "must validate before writing"
    ))
    .is_err());
}

#[test]
fn editing_a_row_replaces_it_and_a_write_failure_keeps_the_draft() {
    let temp = TempDir::new().expect("tempdir");
    let snapshot = Snapshot {
        value: "old".to_owned(),
        revision: "old revision".to_owned(),
        location: "fixture".to_owned(),
    };
    let mut editor = PathEditor::new(Scope::User, snapshot);
    editor.edit(Some(0));
    if let Mode::Edit { input, .. } = &mut editor.mode {
        input.set_text(&temp.path().to_string_lossy());
    }
    accept_entry(&mut editor);
    assert_eq!(editor.entries.len(), 1);
    assert_eq!(editor.entries[0].validation, Validation::Directory);
    let error = save_with(&mut editor, |_, _, _| {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "fixture denied",
        ))
    })
    .expect_err("write refused");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(editor.snapshot.revision, "old revision");
    assert!(editor.dirty());
}

#[test]
fn accept_entry_and_open_handle_browse_separator_new_row_and_scopes() {
    let temp = TempDir::new().expect("tempdir");
    let snapshot = Snapshot {
        value: String::new(),
        revision: "r1".to_owned(),
        location: "fixture".to_owned(),
    };
    let mut editor = PathEditor::new(Scope::User, snapshot);
    accept_entry(&mut editor);
    editor.edit(None);
    if let Mode::Edit { input, .. } = &mut editor.mode {
        input.set_text(&format!("a{}b", crate::path_env::separator()));
    }
    accept_entry(&mut editor);
    assert!(editor.message.contains("one directory"));
    if let Mode::Edit { input, .. } = &mut editor.mode {
        input.set_text(&temp.path().to_string_lossy());
    }
    accept_entry(&mut editor);
    assert_eq!(editor.entries.len(), 1);
    let left = crate::models::Panel::new(temp.path().to_path_buf()).expect("left");
    let right = crate::models::Panel::new(temp.path().to_path_buf()).expect("right");
    let mut app = App::with_panels(left, right);
    drop(open(&mut app, Scope::User));
}

#[test]
fn save_round_trips_user_scope_through_isolated_store() {
    let temp = TempDir::new().expect("tempdir");
    let dir_str = temp.path().to_string_lossy().into_owned();

    #[cfg(unix)]
    {
        let _home = crate::test_support::EnvVarGuard::set("HOME", temp.path());
        let snap = store::read(Scope::User).expect("read");
        let mut editor = PathEditor::new(Scope::User, snap);
        editor.entries = vec![PathEntry {
            text: dir_str.clone(),
            validation: Validation::Directory,
        }];
        save(&mut editor).expect("save user path");
        assert_eq!(editor.snapshot.value, dir_str);
    }

    #[cfg(windows)]
    store::with_fake_user(None, || {
        let snap = store::read(Scope::User).expect("read fake");
        let mut editor = PathEditor::new(Scope::User, snap);
        editor.entries = vec![PathEntry {
            text: dir_str.clone(),
            validation: Validation::Directory,
        }];
        save(&mut editor).expect("save fake user path");
        assert_eq!(editor.snapshot.value, dir_str);
    });
}
