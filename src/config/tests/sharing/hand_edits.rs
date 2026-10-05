//! Unit tests: a change made in the file while FFM runs — by hand or by
//! another FFM — survives a save, and one made on both sides refuses it.

use crate::config::Enablement;

use super::*;

/// The exit auto-save used to put back what the user changed in the file
/// while FFM ran, in its own editor included.
#[test]
fn a_hand_edit_made_while_ffm_runs_survives_the_exit_auto_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut loaded = Settings::load_from(&path)?;
    write_file(&path, "system:\n  history_size: 9\n")?;
    loaded.save_to(&path)?;
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 9\n");
    loaded.system.wipe_on_delete = true;
    loaded.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.system.history_size, 9);
    assert!(saved.system.wipe_on_delete);
    Ok(())
}

#[test]
fn a_setting_changed_on_disk_and_in_the_session_is_not_saved() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut loaded = Settings::load_from(&path)?;
    write_file(&path, "system:\n  history_size: 9\n")?;
    loaded.system.history_size = 7;
    let error = loaded.save_to(&path).expect_err("both changed it");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml changed on disk: system.history_size was edited there \
         and in this session"
    );
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 9\n");
    Ok(())
}

/// A save compares with what was loaded, not with what the last save wrote:
/// a value this session saved and the user then edited by hand stops the
/// next save instead of being overwritten.
#[test]
fn a_hand_edit_of_a_value_this_session_saved_stops_the_next_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut settings = Settings::load_from(&path)?;
    settings.system.history_size = 7;
    settings.save_to(&path)?;
    assert_eq!(Settings::load_from(&path)?.system.history_size, 7);
    write_file(&path, "system:\n  history_size: 9\n")?;
    assert!(settings.save_to(&path).is_err());
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 9\n");
    Ok(())
}

/// A writer whose load failed holds the defaults. Once the user fixes the
/// file, its save writes what it changed and keeps what the user wrote.
#[test]
fn a_writer_whose_load_failed_keeps_what_the_user_fixed() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system: [\n")?;
    assert!(Settings::load_from(&path).is_err());
    let mut fallback = Settings::default();
    fallback.interface.clock = !fallback.interface.clock;
    write_file(
        &path,
        "system: {history_size: 9}\nlsp: {python: {enabled: false}}\n",
    )?;
    fallback.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.system.history_size, 9);
    assert_eq!(saved.lsp.0["python"].enabled, Enablement::Disabled);
    assert_eq!(saved.interface.clock, fallback.interface.clock);
    Ok(())
}

#[test]
fn two_ffms_saving_keep_each_others_changes() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut first = Settings::load_from(&path)?;
    let mut second = Settings::load_from(&path)?;
    second.system.wipe_on_delete = true;
    second.save_to(&path)?;
    first.panel.show_hidden = false;
    first.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.system.history_size, 5);
    assert!(saved.system.wipe_on_delete);
    assert!(!saved.panel.show_hidden);
    Ok(())
}
