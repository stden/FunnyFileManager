use super::*;

#[test]
fn user_menu_and_associations_read_saved_configuration() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let mut app = App::new()?;
    app.settings = crate::models::settings::Settings::with_file(temp.path().join("settings.json"));
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
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    edit(&mut app, MenuCommand::UserMenu)?;
    let config = temp.path().join(".ffm/user-menu.txt");
    fs::write(&config, "Custom\techo custom\n")?;
    edit(&mut app, MenuCommand::UserMenu)?;
    assert_eq!(fs::read_to_string(config)?, "Custom\techo custom\n");
    Ok(())
}
