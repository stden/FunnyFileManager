use super::*;

#[test]
fn user_menu_and_associations_read_saved_configuration() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let mut app = App::new()?;
    app.settings = crate::models::settings::Settings::with_file(temp.path().join("settings.yaml"));
    fs::write(temp.path().join("user-menu.txt"), "Show directory\tpwd\n")?;
    open(&mut app)?;
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("user menu");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(rows[0].label, "Show directory");
    edit(&mut app, MenuCommand::Associations)?;
    assert!(matches!(app.overlay, Overlay::Editor(_)));
    assert!(temp.path().join("associations.txt").exists());
    Ok(())
}

#[test]
fn malformed_command_files_are_filtered_and_unreadable_files_report_errors() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("commands.txt");
    fs::write(
        &path,
        "# comment\ninvalid\n\tmissing label\nEmpty\t \n Run \t echo ok \n",
    )?;
    assert_eq!(
        read_pairs(&path)?,
        vec![("Run".to_owned(), "echo ok".to_owned())]
    );
    assert!(read_pairs(temp.path()).is_err());
    let mut app = app_with_config(temp.path());
    edit(&mut app, MenuCommand::UserMenu)?;
    let config = temp.path().join("user-menu.txt");
    fs::write(&config, "Custom\techo custom\n")?;
    edit(&mut app, MenuCommand::UserMenu)?;
    assert_eq!(fs::read_to_string(config)?, "Custom\techo custom\n");
    Ok(())
}

/// Without a configuration directory the user menu lists no command and
/// editing it says why; nothing is written into the folder on screen.
#[test]
fn without_a_configuration_directory_there_is_no_command_file() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    open(&mut app)?;
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("user menu");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    let labels: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["Edit user menu"]);
    for command in [MenuCommand::UserMenu, MenuCommand::Associations] {
        let error = edit(&mut app, command).expect_err("no configuration directory");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(error.to_string(), NO_CONFIG_DIR);
    }
    assert_eq!(associated_command(&app)?, None);
    assert_eq!(fs::read_dir(temp.path())?.count(), 0, "nothing written");
    Ok(())
}

/// A FIFO named `user-menu.txt` or `associations.txt`: opening it waited for
/// a writer, and F2, Enter on any file or editing the file froze FFM. Each is
/// refused, promptly, with the file's name.
#[cfg(unix)]
#[test]
fn a_command_file_that_is_not_a_regular_file_is_refused_without_waiting() {
    let temp = tempfile::TempDir::new().expect("dir");
    for name in ["user-menu.txt", "associations.txt"] {
        crate::test_support::make_fifo(&temp.path().join(name)).expect("fifo");
    }
    fs::write(temp.path().join("note.txt"), "x").expect("file");
    let config = temp.path().to_owned();
    let outcome =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            let mut app = app_with_config(&config);
            assert!(app.get_active_panel().select_named("note.txt"));
            [
                open(&mut app),
                associated_command(&app).map(drop),
                edit(&mut app, MenuCommand::UserMenu),
            ]
            .map(|result| result.map_err(|error| error.to_string()))
        })
        .expect("no command file is waited on");
    assert_eq!(
        outcome,
        [
            Err("user-menu.txt is not a regular file".to_owned()),
            Err("associations.txt is not a regular file".to_owned()),
            Err("user-menu.txt is not a regular file".to_owned()),
        ]
    );
}

/// An app whose settings live in `config`, both panels on it.
fn app_with_config(config: &Path) -> App {
    let panel = crate::models::Panel::new(config.to_owned()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    app.settings = crate::models::settings::Settings::with_file(config.join("settings.yaml"));
    app
}
