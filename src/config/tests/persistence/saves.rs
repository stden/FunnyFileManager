//! Unit tests: what a save of the typed settings keeps — the Options pages and
//! keys it does not know — and the saves that must not happen.

use super::*;

#[test]
fn a_save_keeps_the_options_pages_and_keys_it_does_not_know() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "options:\n  Editor: '8'\nfrom_a_newer_build: 1\n")?;
    let mut settings = Settings::load_from(&path)?;
    settings.system.wipe_on_delete = true;
    settings.save_to(&path)?;
    let text = fs::read_to_string(&path)?;
    assert!(text.contains("\noptions:\n  Editor: '8'\n"), "{text}");
    assert!(text.contains("\nfrom_a_newer_build: 1\n"), "{text}");
    assert!(text.contains("\nsystem:\n"), "{text}");
    assert!(!text.contains("\nlsp:"), "only the changed page is written");
    Ok(())
}

/// One bad value makes the load fall back to the defaults; saving those
/// would wipe the user's language servers along with the typo.
#[test]
fn a_save_never_replaces_pages_that_do_not_parse() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    let text = "system:\n  history_size: many\nlsp:\n  python:\n    enabled: false\n";
    write_file(&path, text)?;
    assert!(Settings::load_from(&path).is_err());
    let error = Settings::default()
        .save_to(&path)
        .expect_err("the stored pages do not parse");
    let message = error.to_string();
    assert!(message.contains("system.history_size"), "{message}");
    assert!(
        message.starts_with("Not saved: settings.yaml does not parse: "),
        "{message}"
    );
    assert_eq!(fs::read_to_string(&path)?, text);
    Ok(())
}

/// A path that is not valid UTF-8 has no YAML spelling: the save fails before
/// anything is written.
#[cfg(unix)]
#[test]
fn a_shortcut_yaml_cannot_spell_leaves_the_file_untouched() -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut settings = Settings::default();
    settings.shortcuts[0] = Some(std::ffi::OsStr::from_bytes(b"/tmp/\xff").into());
    assert!(settings.save_to(&path).is_err());
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 5\n");
    Ok(())
}
